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
;; Use /(mud/themes) to see available color schemes.
(mud/set-theme "Gruvbox Dark")
;; Use /(mud/fonts) to see available fonts.
;; (mud/set-font "JetBrains Mono")
(mud/set-font-size 14)
(mud/set-scroll-lines 6)

(mud/pane "main")
(mud/on-line (lambda (line) #t))
