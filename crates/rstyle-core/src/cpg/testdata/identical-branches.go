// What identical_branches reports, and what it leaves alone.
package fixture

type Reader struct{ enabled bool }

func (r *Reader) on() bool { return r.enabled }

// Reported: the branch returns what the code after it returns.
func alwaysFalse(r *Reader) bool {
	enabled := !r.on()
	if enabled {
		return false
	}
	return false
}

// Reported: both branches are the same code.
func sameEitherWay(r *Reader, a int) int {
	total := 0
	if r.on() {
		total += a
	} else {
		total += a
	}
	return total
}

// Reported: the branch's two statements are the two after it.
func sameSteps(r *Reader, note func(string)) int {
	if r.on() {
		note("done")
		return 1
	}
	note("done")
	return 1
}

// Reported: redundant, though harmless.
func sameError(run func() error) error {
	err := run()
	if err != nil {
		return err
	}
	return err
}

// Quiet: the condition decides the result.
func decides(r *Reader) bool {
	if r.on() {
		return true
	}
	return false
}

// Quiet: the `v` the condition binds for its branch isn't the `v` after it.
func bindsItsOwn(values map[string]int, key string) int {
	v := 0
	if v, ok := values[key]; ok {
		return v
	}
	return v
}

// Quiet: the branch does more before it returns.
func doesMoreFirst(r *Reader, note func(string)) bool {
	if r.on() {
		note("on")
		return false
	}
	return false
}

// Quiet: the branch doesn't return, so what follows it runs as well.
func runsTwice(r *Reader, note func(string)) {
	if r.on() {
		note("on")
	}
	note("on")
}

// Quiet: another variable.
func otherName(r *Reader, a, b int) int {
	if r.on() {
		return a
	}
	return b
}

// Quiet: only the last of three branches repeats the first.
func threeWays(r *Reader, a, b int) int {
	if r.on() {
		return a
	} else if a > b {
		return b
	} else {
		return a
	}
}

// Reported: each side declares its own `v`, and the two pair up.
func ownCopies(r *Reader, values map[string]int, key string) int {
	if r.on() {
		if v, ok := values[key]; ok {
			return v
		}
		return 0
	}
	if v, ok := values[key]; ok {
		return v
	}
	return 0
}

// Quiet: an empty branch runs nothing.
func emptyBranch(r *Reader) bool {
	if r.on() {
	}
	return false
}

// Quiet: what follows an `else if` comes after the whole chain, which isn't compared.
func elseIf(r *Reader, a, b int) int {
	if r.on() {
		return a
	} else if a > b {
		return b
	}
	return b
}
