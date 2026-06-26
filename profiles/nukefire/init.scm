(define name "NukeFire")
(define host "tdome.nukefire.org")
(define port 4000)

;; Enter your name and password here to log in automatically on connect.
;; Leave empty to log in manually.
(define character "")
(define password "")

;; Options
(option "keep_input" #t)
(option "font" "JetBrains Mono")
(option "font_size" 14)
(option "scroll_lines" 3)

;; 550+ built-in themes from https://iterm2colorschemes.com
(load-theme "Gruvbox Dark")

;; Movement: Alt+WASD + Alt+Q/E
(keymap "alt+w" "n")
(keymap "alt+s" "s")
(keymap "alt+a" "w")
(keymap "alt+d" "e")
(keymap "alt+q" "d")
(keymap "alt+e" "u")

;; Scrolling
(keymap "PageUp" "scroll_up 20")
(keymap "PageDown" "scroll_down 20")

;; ----------------------------------------------------------------

(pane "main")
(pane "map")

(layout "horizontal" (list
    (list "main" 1)
    (list "map" 1)))

(gauge "health" (hash 'color "green"))
(gauge "mana" (hash 'color "cyan"))
(gauge "moves" (hash 'color "blue"))

(define nf (hash))

(define (update-gauges)
  (when (and (hash-contains? nf 'hp) (hash-contains? nf 'hp-max))
    (gauge "health" (hash 'current (hash-ref nf 'hp) 'max (hash-ref nf 'hp-max) 'color "green")))
  (when (and (hash-contains? nf 'mana) (hash-contains? nf 'mana-max))
    (gauge "mana" (hash 'current (hash-ref nf 'mana) 'max (hash-ref nf 'mana-max) 'color "cyan")))
  (when (and (hash-contains? nf 'mv) (hash-contains? nf 'mv-max))
    (gauge "moves" (hash 'current (hash-ref nf 'mv) 'max (hash-ref nf 'mv-max) 'color "blue"))))

(define (update-map-header)
  (pane-clear "map")
  (for-each (lambda (l) (pane-print "map" l))
            (hash-get nf 'map-lines '()))
  (when (hash-contains? nf 'room-info)
    (pane-print "map" "")
    (pane-print "map" (hash-ref nf 'room-info))))

(define (update-status)
  (let ((room (hash-get nf 'room "?"))
        (area (hash-get nf 'area "?"))
        (level (hash-get nf 'level "?"))
        (tnl (hash-get nf 'tnl "?"))
        (exits (hash-get nf 'exits "?")))
    (status (to-string room "   " area "   Lv:" (to-string level) " TNL:" (to-string tnl) "   [" exits "]"))))

(define (on-msdp data)
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
      (update-status))))

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
         (regexp-match? "^[\\s\\-|@*X^v<>:/=!]*$" stripped))))

(define (map-content? text)
  (or (equal? text "")
      (map-grid? text)
      (starts-with? text "[ BIGMAP ]")
      (regexp-match? "Zone:.+Room:" text)
      (starts-with? text "Route:")
      (starts-with? text "@ you")
      (regexp-match? "^\\s*<--" text)))

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
  (for-each (lambda (raw) (pane-print "main" raw)) pending-blanks)
  (set! pending-blanks '()))

(define (on-line line)
  (let ((text (strip-ansi line)))
    (cond
      ((regexp-match? "^> ?[a-zA-Z]{1,2}$" text) #f)
      (else (process-line line text)))))

(define (process-line line text)
  (cond
    ((equal? parse-state "pass")
     (cond
       ((equal? text "")
        (set! pending-blanks (append pending-blanks (list line)))
        #f)
       ((regexp-match? "^[a-zA-Z].+ - \\[" text)
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
       ((regexp-match? "^\\s*<--" text)
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

(define (on-input cmd)
  (let ((trimmed (trim cmd)))
    (when (not (equal? trimmed ""))
      (pane-print "main" (to-string "\x1b;[32m> " trimmed "\x1b;[0m")))))

(define (on-connect)
  (pane-print "main" "[Connected to NukeFire]")
  (let ((msdp-vars (list
          "ROOM_NAME" "ROOM_VNUM" "AREA_NAME" "ROOM_EXITS"
          "HEALTH" "HEALTH_MAX" "MANA" "MANA_MAX"
          "MOVEMENT" "MOVEMENT_MAX" "LEVEL" "EXPERIENCE_TNL")))
    (timer 0.5 (lambda ()
      (msdp-report msdp-vars)
      (msdp-send msdp-vars))))
  (when (not (equal? character ""))
    (timer 0.5 (lambda () (send character)))
    (timer 1.0 (lambda () (send password)))))

(define (on-disconnect)
  (pane-print "main" "[Disconnected from NukeFire]"))
