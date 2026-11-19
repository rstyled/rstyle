// Package loops leaves a loop with no guard by breaking out of it.
package loops

// Forever counts up to limit in a loop with no guard.
func Forever(limit uint) uint {
	var i uint
	for {
		if i >= limit {
			break
		}
		i++
	}
	return i
}
