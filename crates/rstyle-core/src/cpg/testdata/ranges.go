// Package ranges counts a loop over an integer.
package ranges

// Counted ranges over an integer.
func Counted(n int) int {
	total := 0
	for i := range n {
		total += i
	}
	return total
}
