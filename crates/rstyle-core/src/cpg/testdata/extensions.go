// Package extensions slices a slice inside scans of another.
package extensions

// Tails slices ys once per item of xs.
func Tails(xs []int, ys []int) int {
	total := 0
	for _, x := range xs {
		total += len(ys[x:])
	}
	return total
}

// Heads slices ys with all three bounds once per item of xs.
func Heads(xs []int, ys []int) int {
	total := 0
	for _, x := range xs {
		total += cap(ys[0:x:x])
	}
	return total
}
