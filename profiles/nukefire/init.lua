name = "NukeFire"
host = "tdome.nukefire.org"
port = 4000

-- Enter your name and password here to log in automatically on connect.
-- Leave empty to log in manually.
character = ""
password = ""

-- Options
mud.option("keep_input", true)
mud.option("font", "JetBrains Mono")
mud.option("font_size", 14)
mud.option("scroll_lines", 3)

-- 550+ built-in themes from https://iterm2colorschemes.com
mud.load_theme("Gruvbox Dark")

-- Movement: Alt+WASD + Alt+Q/E
mud.keymap("alt+w", "n")
mud.keymap("alt+s", "s")
mud.keymap("alt+a", "w")
mud.keymap("alt+d", "e")
mud.keymap("alt+q", "d")
mud.keymap("alt+e", "u")

-- Scrolling
mud.keymap("PageUp", "scroll_up 20")
mud.keymap("PageDown", "scroll_down 20")

----------------------------------------------------------------

local main = mud.pane("main")
local map = mud.pane("map")

mud.layout("horizontal", {
    { pane = "main", weight = 1 },
    { pane = "map", weight = 1 },
})

mud.gauge("health", { color = "green" })
mud.gauge("mana", { color = "cyan" })
mud.gauge("moves", { color = "blue" })

local nf = {}

local function update_gauges()
    if nf.hp and nf.hp_max then
        mud.gauge("health", { current = tonumber(nf.hp), max = tonumber(nf.hp_max), color = "green" })
    end
    if nf.mana and nf.mana_max then
        mud.gauge("mana", { current = tonumber(nf.mana), max = tonumber(nf.mana_max), color = "cyan" })
    end
    if nf.mv and nf.mv_max then
        mud.gauge("moves", { current = tonumber(nf.mv), max = tonumber(nf.mv_max), color = "blue" })
    end
end

local function update_map_header()
    map:clear()
    for _, l in ipairs(nf.map_lines or {}) do
        map:print(l)
    end
    if nf.room_info then
        map:print("")
        map:print(nf.room_info)
    end
end

local function update_status()
    local room = nf.room or "?"
    local area = nf.area or "?"
    local level = nf.level or "?"
    local tnl = nf.tnl or "?"
    local exits = nf.exits or "?"
    mud.status(room .. "   " .. area .. "   Lv:" .. level .. " TNL:" .. tnl .. "   [" .. exits .. "]")
end

function on_msdp(data)
    if type(data) ~= "table" then return end
    local changed = false
    if data.ROOM_NAME then nf.room = data.ROOM_NAME changed = true end
    if data.AREA_NAME then nf.area = data.AREA_NAME changed = true end
    if data.HEALTH then nf.hp = data.HEALTH changed = true end
    if data.HEALTH_MAX then nf.hp_max = data.HEALTH_MAX changed = true end
    if data.MANA then nf.mana = data.MANA changed = true end
    if data.MANA_MAX then nf.mana_max = data.MANA_MAX changed = true end
    if data.MOVEMENT then nf.mv = data.MOVEMENT changed = true end
    if data.MOVEMENT_MAX then nf.mv_max = data.MOVEMENT_MAX changed = true end
    if data.LEVEL then nf.level = data.LEVEL changed = true end
    if data.EXPERIENCE_TNL then nf.tnl = data.EXPERIENCE_TNL changed = true end
    if data.ROOM_EXITS then
        if type(data.ROOM_EXITS) == "table" then
            local dirs = {}
            for k, _ in pairs(data.ROOM_EXITS) do
                dirs[#dirs + 1] = k
            end
            nf.exits = table.concat(dirs, " ")
        else
            nf.exits = tostring(data.ROOM_EXITS)
        end
        changed = true
    end
    if changed then
        update_gauges()
        update_map_header()
        update_status()
    end
end

local map_lines_raw = {}
local dir_indicators = {}
local room_info_lines = {}
local state = "pass"
local room_buf = {}
local map_buf = {}
local map_has_header = false
local lines_after_player = nil
local pending_blanks = {}

local function is_map_grid(text)
    local stripped = string.gsub(text, "\xe2\x96\xa0", "")
    return string.match(stripped, "^[%s%-|@%*X%^v<>:/=!]*$") ~= nil and text ~= ""
end

local function is_map_content(text)
    return text == ""
        or is_map_grid(text)
        or string.match(text, "^%[ BIGMAP %]") ~= nil
        or string.match(text, "Zone:.+Room:") ~= nil
        or string.match(text, "^Route:") ~= nil
        or string.match(text, "^@ you") ~= nil
        or string.match(text, "^%s*<%-%-") ~= nil
end

local function write_map()
    nf.map_lines = {}
    for _, l in ipairs(dir_indicators) do
        nf.map_lines[#nf.map_lines + 1] = l
    end
    for _, l in ipairs(map_lines_raw) do
        nf.map_lines[#nf.map_lines + 1] = l
    end
    if #room_info_lines > 0 then
        nf.room_info = table.concat(room_info_lines, "\n")
    end
    update_map_header()
end

local function flush_map()
    if map_has_header then
        map_lines_raw = {}
        for _, l in ipairs(map_buf) do
            map_lines_raw[#map_lines_raw + 1] = l
        end
        write_map()
    end
    map_buf = {}
    map_has_header = false
    lines_after_player = nil
    state = "pass"
end

local function emit_blanks()
    for _, raw in ipairs(pending_blanks) do
        main:print(raw)
    end
    pending_blanks = {}
end

function on_line(line)
    local text = mud.strip_ansi(line)

    if string.match(text, "^> ?%a%a?$") then return false end

    local reeval = true
    while reeval do
        reeval = false
        if state == "pass" then
            if text == "" then
                pending_blanks[#pending_blanks + 1] = line
                return false
            elseif string.match(text, "^%a.+ %- %[") then
                state = "room"
                pending_blanks = {}
                room_buf = { line }
                return false
            elseif string.match(text, "^%[ BIGMAP %]") then
                state = "map"
                map_has_header = true
                pending_blanks = {}
                dir_indicators = {}
                map_buf = { line }
                return false
            elseif string.match(text, "^%s*<%-%-") then
                pending_blanks = {}
                dir_indicators[#dir_indicators + 1] = line
                write_map()
                return false
            elseif is_map_grid(text) then
                state = "map"
                pending_blanks = {}
                map_buf = { line }
                return false
            else
                emit_blanks()
                return true
            end
        elseif state == "room" then
            room_buf[#room_buf + 1] = line
            if string.match(text, "^%[ Exits:") then
                room_info_lines = {}
                for _, r in ipairs(room_buf) do
                    room_info_lines[#room_info_lines + 1] = r
                end
                write_map()
                room_buf = {}
                state = "pass"
            end
            return false
        elseif state == "map" then
            if is_map_content(text) then
                map_buf[#map_buf + 1] = line
                if is_map_grid(text) and string.find(text, "@") then
                    lines_after_player = 0
                elseif lines_after_player and is_map_grid(text) then
                    lines_after_player = lines_after_player + 1
                    if lines_after_player >= 6 then
                        flush_map()
                        return false
                    end
                end
                return false
            else
                flush_map()
                reeval = true
            end
        end
    end

    return true
end

function on_input(cmd)
    local trimmed = string.gsub(string.gsub(cmd, "^%s+", ""), "%s+$", "")
    if trimmed ~= "" then
        main:print("\x1b[32m> " .. trimmed .. "\x1b[0m")
    end
end

function on_connect()
    main:print("[Connected to NukeFire]")
    local msdp_vars = {
        "ROOM_NAME", "ROOM_VNUM", "AREA_NAME", "ROOM_EXITS",
        "HEALTH", "HEALTH_MAX", "MANA", "MANA_MAX",
        "MOVEMENT", "MOVEMENT_MAX", "LEVEL", "EXPERIENCE_TNL",
    }
    mud.timer(0.5, function()
        mud.msdp_report(msdp_vars)
        mud.msdp_send(msdp_vars)
    end)
    if character ~= "" then
        mud.timer(0.5, function() mud.send(character) end)
        mud.timer(1.0, function() mud.send(password) end)
    end
end

function on_disconnect()
    main:print("[Disconnected from NukeFire]")
end
