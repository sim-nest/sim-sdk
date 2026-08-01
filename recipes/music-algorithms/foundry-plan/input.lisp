(load "sim-lib-numbers-signal")
(load "sim-lib-discrete-search")
(load "sim-lib-pitch-ratio")
(load "sim-lib-music-consonance")
(load "sim-lib-music-counterpoint")

(define plan
  (music/algorithm-plan
    :input (storage/get "song.mid")
    :analysis '(pitch-track beat key chords)
    :transform '(voice-lead harmonize counterpoint)
    :render '(smf wav)
    :budget {:work 500000 :frontier 20000 :results 8 :seed 42}
    :deadline-ms 5000))

(realize plan :at 'local)
