;; Steel Scheme (R5RS subset) — https://github.com/mattwparas/steel

(mud/profile
  'name "NukeFire"
  'host "tdome.nukefire.org"
  'port 4000
  'tls #f)

;; Enter your name and password here to log in automatically on connect.
;; Leave empty to log in manually.
(define character "")
(define password "")

;; Options
(mud/set-keep-input #t)
;; Use /(mud/fonts) to see available fonts.
;; (mud/set-font "JetBrains Mono")
(mud/set-font-size 14)
(mud/set-scroll-lines 6)

;; Use /(mud/themes) to see available color schemes.
(mud/set-theme "Gruvbox Dark")

;; Movement: Alt+WASD + Alt+Q/E
(mud/keymap "alt+w" (lambda () (mud/send "n")))
(mud/keymap "alt+s" (lambda () (mud/send "s")))
(mud/keymap "alt+a" (lambda () (mud/send "w")))
(mud/keymap "alt+d" (lambda () (mud/send "e")))
(mud/keymap "alt+q" (lambda () (mud/send "d")))
(mud/keymap "alt+e" (lambda () (mud/send "u")))

;; Scrolling
(mud/keymap "PageUp" (lambda () (mud/scroll-up 20)))
(mud/keymap "PageDown" (lambda () (mud/scroll-down 20)))

;; ----------------------------------------------------------------

(mud/pane "main")
(mud/pane "map")

(mud/layout "horizontal" (list
    (list "main" 1)
    (list "map" 1)))

(mud/gauge "health" (hash 'color "green"))
(mud/gauge "mana" (hash 'color "cyan"))
(mud/gauge "moves" (hash 'color "blue"))

(define nf (hash))

(define (update-gauges)
  (when (and (hash-contains? nf 'hp) (hash-contains? nf 'hp-max))
    (mud/gauge "health" (hash 'current (hash-ref nf 'hp) 'max (hash-ref nf 'hp-max) 'color "green")))
  (when (and (hash-contains? nf 'mana) (hash-contains? nf 'mana-max))
    (mud/gauge "mana" (hash 'current (hash-ref nf 'mana) 'max (hash-ref nf 'mana-max) 'color "cyan")))
  (when (and (hash-contains? nf 'mv) (hash-contains? nf 'mv-max))
    (mud/gauge "moves" (hash 'current (hash-ref nf 'mv) 'max (hash-ref nf 'mv-max) 'color "blue"))))

(define (update-map-header)
  (mud/pane-clear "map")
  (for-each (lambda (l) (mud/pane-print "map" l))
            (hash-get nf 'map-lines '()))
  (when (hash-contains? nf 'room-info)
    (mud/pane-print "map" "")
    (mud/pane-print "map" (hash-ref nf 'room-info))))

(define (update-status)
  (let ((room (hash-get nf 'room "?"))
        (area (hash-get nf 'area "?"))
        (level (hash-get nf 'level "?"))
        (tnl (hash-get nf 'tnl "?"))
        (exits (hash-get nf 'exits "?")))
    (mud/status (to-string room "   " area "   Lv:" (to-string level) " TNL:" (to-string tnl) "   [" exits "]"))))

(mud/on-msdp (lambda (data)
  (when (hash? data)
    (define changed #f)
    (when (hash-contains? data "ROOM_NAME") (set! nf (hash-insert nf 'room (hash-ref data "ROOM_NAME"))) (set! changed #t))
    (when (hash-contains? data "AREA_NAME") (set! nf (hash-insert nf 'area (hash-ref data "AREA_NAME"))) (set! changed #t))
    (when (hash-contains? data "HEALTH") (set! nf (hash-insert nf 'hp (hash-ref data "HEALTH"))) (set! changed #t))
    (when (hash-contains? data "HEALTH_MAX") (set! nf (hash-insert nf 'hp-max (hash-ref data "HEALTH_MAX"))) (set! changed #t))
    (when (hash-contains? data "MANA") (set! nf (hash-insert nf 'mana (hash-ref data "MANA"))) (set! changed #t))
    (when (hash-contains? data "MANA_MAX") (set! nf (hash-insert nf 'mana-max (hash-ref data "MANA_MAX"))) (set! changed #t))
    (when (hash-contains? data "MOVEMENT") (set! nf (hash-insert nf 'mv (hash-ref data "MOVEMENT"))) (set! changed #t))
    (when (hash-contains? data "MOVEMENT_MAX") (set! nf (hash-insert nf 'mv-max (hash-ref data "MOVEMENT_MAX"))) (set! changed #t))
    (when (hash-contains? data "LEVEL") (set! nf (hash-insert nf 'level (hash-ref data "LEVEL"))) (set! changed #t))
    (when (hash-contains? data "EXPERIENCE_TNL") (set! nf (hash-insert nf 'tnl (hash-ref data "EXPERIENCE_TNL"))) (set! changed #t))
    (when (hash-contains? data "ROOM_EXITS")
      (let ((exits (hash-ref data "ROOM_EXITS")))
        (set! nf (hash-insert nf 'exits
          (if (hash? exits)
              (string-join (hash-keys->list exits) " ")
              (to-string exits))))
        (set! changed #t)))
    (when changed
      (update-gauges)
      (update-map-header)
      (update-status)))))

(define map-lines-raw '())
(define dir-indicators '())
(define room-info-lines '())
(define parse-state "pass")
(define room-buf '())
(define map-buf '())
(define map-has-header #f)
(define lines-after-player #f)
(define pending-blanks '())

(define (map-grid? text)
  (and (not (equal? text ""))
       (let ((stripped (string-replace text "■" "")))
         (mud/regexp-match? "^[\\s\\-|@*X^v<>:/=!]*$" stripped))))

(define (map-content? text)
  (or (equal? text "")
      (map-grid? text)
      (starts-with? text "[ BIGMAP ]")
      (mud/regexp-match? "Zone:.+Room:" text)
      (starts-with? text "Route:")
      (starts-with? text "@ you")
      (mud/regexp-match? "^\\s*<--" text)))

(define (write-map)
  (set! nf (hash-insert nf 'map-lines (append dir-indicators map-lines-raw)))
  (when (not (null? room-info-lines))
    (set! nf (hash-insert nf 'room-info (string-join room-info-lines "\n"))))
  (update-map-header))

(define (flush-map)
  (when map-has-header
    (set! map-lines-raw map-buf)
    (write-map))
  (set! map-buf '())
  (set! map-has-header #f)
  (set! lines-after-player #f)
  (set! parse-state "pass"))

(define (emit-blanks)
  (for-each (lambda (raw) (mud/pane-print "main" raw)) pending-blanks)
  (set! pending-blanks '()))

(mud/on-line (lambda (line)
  (let ((text (mud/strip-ansi line)))
    (cond
      ((mud/regexp-match? "^> ?[a-zA-Z]{1,2}$" text) #f)
      (else (process-line line text))))))

(define (process-line line text)
  (cond
    ((equal? parse-state "pass")
     (cond
       ((equal? text "")
        (set! pending-blanks (append pending-blanks (list line)))
        #f)
       ((mud/regexp-match? "^[a-zA-Z].+ - \\[" text)
        (set! parse-state "room")
        (set! pending-blanks '())
        (set! room-buf (list line))
        #f)
       ((starts-with? text "[ BIGMAP ]")
        (set! parse-state "map")
        (set! map-has-header #t)
        (set! pending-blanks '())
        (set! dir-indicators '())
        (set! map-buf (list line))
        #f)
       ((mud/regexp-match? "^\\s*<--" text)
        (set! pending-blanks '())
        (set! dir-indicators (append dir-indicators (list line)))
        (write-map)
        #f)
       ((map-grid? text)
        (set! parse-state "map")
        (set! pending-blanks '())
        (set! map-buf (list line))
        #f)
       (else
        (emit-blanks)
        #t)))
    ((equal? parse-state "room")
     (set! room-buf (append room-buf (list line)))
     (when (starts-with? text "[ Exits:")
       (set! room-info-lines room-buf)
       (write-map)
       (set! room-buf '())
       (set! parse-state "pass"))
     #f)
    ((equal? parse-state "map")
     (cond
       ((map-content? text)
        (set! map-buf (append map-buf (list line)))
        (when (and (map-grid? text) (string-contains? text "@"))
          (set! lines-after-player 0))
        (when (and lines-after-player (map-grid? text))
          (set! lines-after-player (+ lines-after-player 1))
          (when (>= lines-after-player 6)
            (flush-map)))
        #f)
       (else
        (flush-map)
        (process-line line text))))
    (else #t)))

(mud/on-input (lambda (cmd)
  (let ((trimmed (trim cmd)))
    (when (not (equal? trimmed ""))
      (mud/pane-print "main" (to-string "\u{1b}[32m> " trimmed "\u{1b}[0m"))))))

(mud/on-connect (lambda ()
  (mud/pane-print "main" "[Connected to NukeFire]")
  (let ((msdp-vars (list
          "ROOM_NAME" "ROOM_VNUM" "AREA_NAME" "ROOM_EXITS"
          "HEALTH" "HEALTH_MAX" "MANA" "MANA_MAX"
          "MOVEMENT" "MOVEMENT_MAX" "LEVEL" "EXPERIENCE_TNL")))
    (mud/timer 0.5 (lambda ()
      (mud/msdp-report msdp-vars)
      (mud/msdp-send msdp-vars))))
  (when (not (equal? character ""))
    (mud/timer 0.5 (lambda () (mud/send character)))
    (mud/timer 1.0 (lambda () (mud/send password))))))

(mud/on-disconnect (lambda ()
  (mud/pane-print "main" "[Disconnected from NukeFire]")))
