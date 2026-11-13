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

// Quiet: the checked error is returned unchanged.
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

// Quiet: a checked error can accompany a result.
func sameErrorPair(err error, result int) (int, error) {
	if err != nil {
		return result, err
	}
	return result, err
}

// Quiet: comparison order and equality do not change pass-through behavior.
func sameErrorReversed(err error) error {
	if nil == err {
		return err
	} else {
		return err
	}
}

// Quiet: an explicit placeholder between a guard and the repeated return.
func todoGuard(r *Reader) error {
	if r == nil {
		return nil
	}
	// TODO: validate the reader.
	return nil
}

// Reported: the error is swallowed, not passed through.
func droppedError(err error, result int) (int, error) {
	if err != nil {
		return result, nil
	}
	return result, nil
}

// Reported: a TODO elsewhere does not mark this guard as a placeholder.
func unrelatedTodo(r *Reader) bool {
	// TODO: improve the feature lookup.
	if r.on() {
		return false
	}
	return false
}

// Reported: TODO text inside a string is not a placeholder comment.
func todoString(r *Reader) string {
	if r.on() {
		return "TODO"
	}
	return "TODO"
}

// Reported: duplicated work is not just a placeholder return.
func todoWork(r *Reader, note func(string)) int {
	if r.on() {
		note("done")
		return 1
	}
	// TODO: improve this work.
	note("done")
	return 1
}

// Reported: TODOCount is not a TODO marker.
func todoWord(r *Reader) bool {
	if r.on() {
		return false
	}
	// TODOCount is a metric name.
	return false
}

// Reported: checking a call is not a plain binding pass-through.
func checkedCall(run func() error) error {
	if run() != nil {
		return nil
	}
	return nil
}

var sharedError error

// Reported: a global is not a local binding pass-through.
func checkedGlobal() error {
	if sharedError != nil {
		return sharedError
	}
	return sharedError
}

// Reported: duplicate work before a return still matters.
func errorWork(err error, note func(string)) error {
	if err != nil {
		note("done")
		return err
	}
	note("done")
	return err
}

// Reported: work inside a return still matters.
func errorCall(err error, run func() int) (int, error) {
	if err != nil {
		return run(), err
	}
	// TODO: choose a different call.
	return run(), err
}

// Reported: equality between two values is not a nil pass-through guard.
func equalValues(a, b int) int {
	if a == b {
		return a
	}
	return a
}
