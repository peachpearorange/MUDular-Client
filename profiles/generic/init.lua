name = "Generic"
host = "localhost"
port = 4000

local main = mud.pane("main")

function on_connect()
    main:print("[Connected]")
end

function on_disconnect()
    main:print("[Disconnected]")
end

function on_line(line)
    return true
end

function on_gmcp(package, data)
    main:print("[GMCP " .. package .. "]")
end
