// Package callables hands functions to the standard library and calls one where it's written.
package callables

import (
	"os"
	"sync"
)

func count(x int) int { return x + 1 }

func clean() { os.Remove("stale") }

// Cache loads its items the first time they're asked for.
type Cache struct {
	once  sync.Once
	items []int
}

func (c *Cache) load() { c.items = append(c.items, 1) }

// Get hands Do a method by its value.
func (c *Cache) Get() []int {
	c.once.Do(c.load)
	return c.items
}

// Once scans ys for each of xs, and hands Do a scan of xs to run at most once.
func Once(xs, ys []int, once *sync.Once) int {
	total := 0
	for _, x := range xs {
		for _, y := range ys {
			total += x * y
		}
	}
	once.Do(func() {
		for _, x := range xs {
			count(x)
		}
	})
	return total
}

// Named hands Do a function by its name.
func Named(once *sync.Once) {
	once.Do(clean)
}

// Value hands Do a function only its caller knows.
func Value(once *sync.Once, f func()) {
	once.Do(f)
}

// Written hands Do a function that writes what it captures.
func Written(xs []int, once *sync.Once) int {
	total := 0
	once.Do(func() {
		for _, x := range xs {
			total += x
		}
	})
	return total
}

// Immediate calls a function where it's written.
func Immediate(xs []int) int {
	return func() int {
		sum := 0
		for _, x := range xs {
			sum += count(x)
		}
		return sum
	}()
}
