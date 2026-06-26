;;; query.scm — Guile Scheme interface to the bohemia_graph shared library
;;;
;;; Usage:  guile query.scm
;;;
;;; The shared library must be built first:
;;;   cargo build --release
;;; Then the .dylib is at target/release/libbohemia_graph.dylib

(use-modules (system foreign)
             (system foreign-library)
             (ice-9 format))

;; ---------------------------------------------------------------------------
;; Load the shared library
;; ---------------------------------------------------------------------------

(define lib
  (load-foreign-library
   (string-append (dirname (current-filename))
                  "/target/x86_64-apple-darwin/release/libbohemia_graph.dylib")))

;; ---------------------------------------------------------------------------
;; Bind FFI functions
;; ---------------------------------------------------------------------------

(define graph-new
  (foreign-library-function lib "graph_new"
    #:return-type '*
    #:arg-types '()))

(define graph-load-raw
  (foreign-library-function lib "graph_load"
    #:return-type int
    #:arg-types (list '* '* '* '* '* int)))

(define graph-destroy
  (foreign-library-function lib "graph_destroy"
    #:return-type void
    #:arg-types '(*)))

(define graph-free-str
  (foreign-library-function lib "graph_free_str"
    #:return-type void
    #:arg-types '(*)))

(define graph-get-raw
  (foreign-library-function lib "graph_get"
    #:return-type '*
    #:arg-types '(* *)))

(define graph-describe-raw
  (foreign-library-function lib "graph_describe"
    #:return-type '*
    #:arg-types '(* *)))

(define graph-edges-from-raw
  (foreign-library-function lib "graph_edges_from"
    #:return-type '*
    #:arg-types '(* * * *)))

(define graph-edges-to-raw
  (foreign-library-function lib "graph_edges_to"
    #:return-type '*
    #:arg-types '(* * * *)))

(define graph-bfs-raw
  (foreign-library-function lib "graph_bfs"
    #:return-type '*
    #:arg-types (list '* '* int '*)))

(define graph-node-count
  (foreign-library-function lib "graph_node_count"
    #:return-type int
    #:arg-types '(*)))

;; ---------------------------------------------------------------------------
;; Helpers: C string <-> Scheme string
;; ---------------------------------------------------------------------------

(define (str->ptr s)
  (string->pointer s "UTF-8"))

(define (ptr->str ptr)
  (if (null-pointer? ptr)
      #f
      (let ((s (pointer->string ptr -1 "UTF-8")))
        (graph-free-str ptr)
        s)))

(define %null %null-pointer)

;; ---------------------------------------------------------------------------
;; High-level wrappers
;; ---------------------------------------------------------------------------

(define (graph-load! handle entities events moments triplets)
  (graph-load-raw handle
                  (str->ptr entities)
                  (str->ptr events)
                  (str->ptr moments)
                  (str->ptr triplets)
                  -1))

(define (graph-describe handle id)
  (ptr->str (graph-describe-raw handle (str->ptr id))))

(define (graph-edges-from handle id . kwargs)
  (let ((pred (if (null? kwargs) %null (str->ptr (car kwargs))))
        (truth (if (or (null? kwargs) (null? (cdr kwargs)))
                   %null (str->ptr (cadr kwargs)))))
    (ptr->str (graph-edges-from-raw handle (str->ptr id) pred truth))))

(define (graph-edges-to handle id . kwargs)
  (let ((pred (if (null? kwargs) %null (str->ptr (car kwargs))))
        (truth (if (or (null? kwargs) (null? (cdr kwargs)))
                   %null (str->ptr (cadr kwargs)))))
    (ptr->str (graph-edges-to-raw handle (str->ptr id) pred truth))))

(define (graph-bfs handle seeds max-hops . kwargs)
  (let* ((seeds-json (format #f "[~a]"
                              (string-join
                               (map (lambda (s) (format #f "\"~a\"" s)) seeds)
                               ",")))
         (truth-ptr (if (null? kwargs) %null (str->ptr (car kwargs)))))
    (ptr->str (graph-bfs-raw handle (str->ptr seeds-json) max-hops truth-ptr))))

;; ---------------------------------------------------------------------------
;; Demo
;; ---------------------------------------------------------------------------

(define base "../ner-20260608")

(define G (graph-new))

(format #t "Loading graph data...~%")
(let ((rc (graph-load! G
                        (string-append base "/bohemia_entities.jsonl")
                        (string-append base "/bohemia_events.jsonl")
                        (string-append base "/bohemia_moments.jsonl")
                        (string-append base "/bohemia_triplets.jsonl"))))
  (if (= rc 0)
      (format #t "Loaded OK — ~a nodes~%" (graph-node-count G))
      (error "graph_load failed" rc)))

(format #t "~%--- describe ---~%")
(format #t "~a~%" (graph-describe G "wiki:Sherlock_Holmes"))
(format #t "~a~%" (graph-describe G "wiki:Irene_Adler"))

(format #t "~%--- edges from Holmes ---~%")
(format #t "~a~%" (graph-edges-from G "wiki:Sherlock_Holmes"))

(format #t "~%--- BFS from Holmes, 2 hops ---~%")
(format #t "~a~%" (graph-bfs G '("wiki:Sherlock_Holmes") 2))

(graph-destroy G)
(format #t "~%Done.~%")
