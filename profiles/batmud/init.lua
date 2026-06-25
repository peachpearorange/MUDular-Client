name = "BatMUD"
host = "batmud.bat.org"
port = 23

local main = mud.pane("main")

mud.gauge("health", { current = 100, max = 100, color = "red" })
mud.gauge("sp", { current = 100, max = 100, color = "blue" })
mud.gauge("ep", { current = 100, max = 100, color = "green" })

function on_connect()
    main:print("[Connected to BatMUD]")
end

function on_disconnect()
    main:print("[Disconnected from BatMUD]")
end

function on_gmcp(package, data)
    if package == "Char.Vitals" then
        if data.hp and data.maxhp then
            mud.gauge("health", { current = data.hp, max = data.maxhp, color = "red" })
        end
        if data.sp and data.maxsp then
            mud.gauge("sp", { current = data.sp, max = data.maxsp, color = "blue" })
        end
        if data.ep and data.maxep then
            mud.gauge("ep", { current = data.ep, max = data.maxep, color = "green" })
        end
    end
end

function on_line(line)
    return true
end
