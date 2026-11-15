package fx

import (
	"database/sql"
	"io"
	"net/http"
	"os"
	"strings"
)

func query(db *sql.DB, w http.ResponseWriter, r *http.Request) {
	db.Exec("delete from t where id = " + r.URL.Query().Get("id"))
}

func path(w http.ResponseWriter, r *http.Request) { os.Open(r.URL.Path) }

func parameterized(db *sql.DB, w http.ResponseWriter, r *http.Request) { db.Exec("delete from t where id = ?", r.FormValue("id")) }

func onlyContext(db *sql.DB, w http.ResponseWriter, r *http.Request) { db.ExecContext(r.Context(), "delete from t") }

func unbounded(w http.ResponseWriter, r *http.Request) { io.ReadAll(r.Body) }

func bounded(w http.ResponseWriter, r *http.Request) { io.ReadAll(http.MaxBytesReader(w, r.Body, 1<<20)) }

func response(c *http.Client) {
	resp, _ := c.Get("https://example.com")
	io.ReadAll(resp.Body)
}

func environment() { io.ReadAll(strings.NewReader(os.Getenv("Q"))) }

func send(c *http.Client, req *http.Request) {
	resp, _ := c.Do(req)
	io.ReadAll(resp.Body)
}

func readBody(r *http.Request) { io.ReadAll(r.Body) }

func handler(w http.ResponseWriter, r *http.Request) { readBody(r) }

var seen = map[string]string{}

func remember(w http.ResponseWriter, r *http.Request) { seen[r.URL.Query().Get("id")] = "" }

func lookup(w http.ResponseWriter, r *http.Request) string { return seen[r.FormValue("id")] }

func asValue(w http.ResponseWriter, r *http.Request) { seen["id"] = r.FormValue("id") }

func byEnvironment() { seen[os.Getenv("Q")] = "" }

func local(w http.ResponseWriter, r *http.Request) int {
	counts := map[string]int{}
	counts[r.FormValue("id")] = 1
	return len(counts)
}

func into(m map[string]string, w http.ResponseWriter, r *http.Request) { m[r.FormValue("id")] = "" }
