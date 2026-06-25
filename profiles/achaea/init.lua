name = "Achaea"
host = "achaea.com"
port = 23

local main = mud.pane("main")
local map = mud.pane("map")

mud.layout("horizontal", {
    { pane = "main", weight = 3 },
    { pane = "map", weight = 1 },
})

mud.gauge("health", { current = 100, max = 100, color = "red" })
mud.gauge("mana", { current = 100, max = 100, color = "blue" })
mud.gauge("endurance", { current = 100, max = 100, color = "green" })

function on_connect()
    main:print("[Connected to Achaea, Dreams of the Divine]")
end

function on_disconnect()
    main:print("[Disconnected from Achaea]")
end

function on_gmcp(package, data)
    if package == "Char.Vitals" then
        if data.hp and data.maxhp then
            mud.gauge("health", { current = tonumber(data.hp), max = tonumber(data.maxhp), color = "red" })
        end
        if data.mp and data.maxmp then
            mud.gauge("mana", { current = tonumber(data.mp), max = tonumber(data.maxmp), color = "blue" })
        end
        if data.ep and data.maxep then
            mud.gauge("endurance", { current = tonumber(data.ep), max = tonumber(data.maxep), color = "green" })
        end
    end
end

local in_map = false
local map_lines = {}

function on_line(line)
    if string.find(line, "^%s*%-%-%-%-%-%-%-%-%+") or string.find(line, "^%s*|.*|%s*$") then
        in_map = true
        map:print(line)
        return false
    end

    if in_map then
        if line == "" or not string.find(line, "[|%-+]") then
            in_map = false
        else
            map:print(line)
            return false
        end
    end

    return true
end

mud.alias("^gg$", function()
    mud.send("get gold from corpse")
end)

mud.alias("^aa (.+)$", function(target)
    mud.send("attack " .. target)
end)
