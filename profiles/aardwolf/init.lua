name = "Aardwolf"
host = "aardmud.org"
port = 23

local main = mud.pane("main")

mud.gauge("health", { current = 100, max = 100, color = "red" })
mud.gauge("mana", { current = 100, max = 100, color = "blue" })
mud.gauge("moves", { current = 100, max = 100, color = "yellow" })

function on_connect()
    main:print("[Connected to Aardwolf MUD]")
end

function on_disconnect()
    main:print("[Disconnected from Aardwolf]")
end

function on_gmcp(package, data)
    if package == "char.vitals" then
        if data.hp and data.maxhp then
            mud.gauge("health", { current = data.hp, max = data.maxhp, color = "red" })
        end
        if data.mana and data.maxmana then
            mud.gauge("mana", { current = data.mana, max = data.maxmana, color = "blue" })
        end
        if data.moves and data.maxmoves then
            mud.gauge("moves", { current = data.moves, max = data.maxmoves, color = "yellow" })
        end
    end
end

function on_line(line)
    return true
end

mud.alias("^sc$", function()
    mud.send("score")
end)
