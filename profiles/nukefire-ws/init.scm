;; Steel implementation of R5RS Scheme

(mud/profile
  'name "NukeFire WebSocket (experimental)"
  'connection-mode 'websocket
  'host "tintin.nukefire.org"
  'port 443
  'tls #f
  'websocket-url "wss://tintin.nukefire.org/ws"
  'websocket-protocol "tty")

;; Uses the same endpoint and tty WebSocket protocol as the browser client.
(mud/load-theme "Gruvbox Dark")
;; (mud/option "font" "JetBrains Mono")
(mud/option "font_size" 14)
(mud/option "scroll_lines" 6)

(mud/pane "main")
(mud/on "line" (lambda (line) #t))
