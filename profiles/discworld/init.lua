name = "Discworld"
host = "discworld.atuin.net"
port = 4242

local main = mud.pane("main")
local map = mud.pane("map")

mud.layout("horizontal", {
    { pane = "main", weight = 3 },
    { pane = "map", weight = 1 },
})

mud.gauge("hp", { current = 100, max = 100, color = "red" })
mud.gauge("gp", { current = 100, max = 100, color = "blue" })
mud.gauge("xp", { current = 0, max = 100, color = "yellow" })

function on_connect()
    main:print("[Connected to Discworld MUD]")
end

function on_disconnect()
    main:print("[Disconnected from Discworld]")
end

function on_gmcp(package, data)
    if package == "char.vitals" then
        if data.hp and data.maxhp then
            mud.gauge("hp", { current = data.hp, max = data.maxhp, color = "red" })
        end
        if data.gp and data.maxgp then
            mud.gauge("gp", { current = data.gp, max = data.maxgp, color = "blue" })
        end
    end
end

local in_map = false

function on_line(line)
    if string.find(line, "^%+[-]+%+$") then
        in_map = not in_map
        map:print(line)
        return false
    end

    if in_map then
        map:print(line)
        return false
    end

    return true
end

mud.alias("^l$", function()
    mud.send("look")
end)
