;;; query.scm — Guile Scheme interface to the bohemia_graph shared library
;;;
;;; Usage:  guile query.scm
;;;
;;; Build the x86_64 release first:
;;;   cargo build --release --target x86_64-apple-darwin

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
;; Bind FFI functions — JSON-returning variants (kept for reference)
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

(define graph-node-count
  (foreign-library-function lib "graph_node_count"
    #:return-type int
    #:arg-types '(*)))

;; ---------------------------------------------------------------------------
;; Bind FFI functions — native SCM-returning variants
;;
;; These return real Scheme values (strings, alists, lists) directly from Rust.
;; Return type is simply '* — Guile treats the usize as an SCM word.
;; ---------------------------------------------------------------------------

(define %graph-node-scm
  (foreign-library-function lib "graph_node_scm"
    #:return-type '*
    #:arg-types '(* *)))

(define %graph-describe-scm
  (foreign-library-function lib "graph_describe_scm"
    #:return-type '*
    #:arg-types '(* *)))

(define %graph-edges-from-scm
  (foreign-library-function lib "graph_edges_from_scm"
    #:return-type '*
    #:arg-types '(* * * *)))

(define %graph-edges-to-scm
  (foreign-library-function lib "graph_edges_to_scm"
    #:return-type '*
    #:arg-types '(* * * *)))

(define %graph-bfs-scm
  (foreign-library-function lib "graph_bfs_scm"
    #:return-type '*
    #:arg-types (list '* '* int '*)))

(define %graph-transitive-closure-scm
  (foreign-library-function lib "graph_transitive_closure_scm"
    #:return-type '*
    #:arg-types '(* * *)))

(define %graph-all-ids-scm
  (foreign-library-function lib "graph_all_ids_scm"
    #:return-type '*
    #:arg-types '(*)))

(define %graph-canonicalize-scm
  (foreign-library-function lib "graph_canonicalize_scm"
    #:return-type '*
    #:arg-types '(*)))

;; ---------------------------------------------------------------------------
;; Helpers
;; ---------------------------------------------------------------------------

(define (str->ptr s)
  (string->pointer s "UTF-8"))

(define %null %null-pointer)

;; The SCM-returning functions hand back a raw pointer that IS the SCM word.
;; We use pointer->scm (Guile 3.x) to reinterpret it as a Scheme value.
(define (ptr->scm p) (pointer->scm p))

;; ---------------------------------------------------------------------------
;; High-level wrappers (SCM-native)
;; ---------------------------------------------------------------------------

(define (graph-load! handle entities events moments triplets)
  (graph-load-raw handle
                  (str->ptr entities)
                  (str->ptr events)
                  (str->ptr moments)
                  (str->ptr triplets)
                  -1))

(define (graph-node handle id)
  (ptr->scm (%graph-node-scm handle (str->ptr id))))

(define (graph-describe handle id)
  (ptr->scm (%graph-describe-scm handle (str->ptr id))))

(define (graph-edges-from handle id . kwargs)
  (let ((pred  (if (null? kwargs)             %null (str->ptr (car kwargs))))
        (truth (if (or (null? kwargs)
                       (null? (cdr kwargs)))  %null (str->ptr (cadr kwargs)))))
    (ptr->scm (%graph-edges-from-scm handle (str->ptr id) pred truth))))

(define (graph-edges-to handle id . kwargs)
  (let ((pred  (if (null? kwargs)             %null (str->ptr (car kwargs))))
        (truth (if (or (null? kwargs)
                       (null? (cdr kwargs)))  %null (str->ptr (cadr kwargs)))))
    (ptr->scm (%graph-edges-to-scm handle (str->ptr id) pred truth))))

(define (graph-bfs handle seeds max-hops . kwargs)
  (let* ((seeds-json (format #f "[~a]"
                              (string-join
                               (map (lambda (s) (format #f "\"~a\"" s)) seeds)
                               ",")))
         (truth-ptr (if (null? kwargs) %null (str->ptr (car kwargs)))))
    (ptr->scm (%graph-bfs-scm handle (str->ptr seeds-json) max-hops truth-ptr))))

(define (graph-transitive-closure handle start pred)
  (ptr->scm (%graph-transitive-closure-scm handle (str->ptr start) (str->ptr pred))))

(define (graph-all-ids handle)
  (ptr->scm (%graph-all-ids-scm handle)))

(define (graph-canonicalize id)
  (ptr->scm (%graph-canonicalize-scm (str->ptr id))))

;; ---------------------------------------------------------------------------
;; Demo
;; ---------------------------------------------------------------------------

(define base (dirname (current-filename)))
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

;; -- describe returns a native Scheme string --
(format #t "~%--- describe (native Scheme strings) ---~%")
(let ((name (graph-describe G "wiki:Sherlock_Holmes")))
  (format #t "string? ~a  value: ~s~%" (string? name) name))

;; -- edges-from returns a list of alists --
(format #t "~%--- edges from Holmes (native alists) ---~%")
(let ((edges (graph-edges-from G "wiki:Sherlock_Holmes")))
  (format #t "~a edges returned~%" (length edges))
  (for-each
   (lambda (edge)
     (format #t "  ~a -> ~a  [~a]~%"
             (assq-ref edge 'subject-id)
             (assq-ref edge 'object-id)
             (assq-ref edge 'predicate)))
   edges))

;; -- bfs returns a list of layers, each a list of ID strings --
(format #t "~%--- BFS from Holmes, 2 hops ---~%")
(let ((layers (graph-bfs G '("wiki:Sherlock_Holmes") 2)))
  (for-each
   (lambda (layer i)
     (format #t "  layer ~a: ~a nodes~%" i (length layer))
     (for-each (lambda (id) (format #t "    ~a~%" id))
               (list-head layer (min 4 (length layer))))
     (when (> (length layer) 4)
       (format #t "    ... and ~a more~%" (- (length layer) 4))))
   layers
   '(0 1 2)))

;; -- node returns a full alist --
(format #t "~%--- node alist for Irene Adler ---~%")
(let ((node (graph-node G "wiki:Irene_Adler")))
  (for-each
   (lambda (pair)
     (format #t "  ~a: ~s~%" (car pair) (cdr pair)))
   node))

(graph-destroy G)
(format #t "~%Done.~%")
