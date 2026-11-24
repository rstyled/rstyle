// Package deferred defers calls inside and around scans.
package deferred

import "sync"

// Locked unlocks once the function returns, after a scan.
func Locked(mu *sync.Mutex, xs []int) int {
	mu.Lock()
	defer mu.Unlock()
	total := 0
	for _, x := range xs {
		total += x
	}
	return total
}

// Closed defers a closure, which runs at the return with the values it sees then.
func Closed(xs []int) int {
	total := 0
	defer func() {
		total++
	}()
	for _, x := range xs {
		total += x
	}
	return total
}

// Each defers a call per item, all of which run at the return.
func Each(wg *sync.WaitGroup, xs []int) {
	for range xs {
		defer wg.Done()
	}
}

// Reported defers a closure that only reads, which still sees the values of the return.
func Reported(xs []int) {
	limit := len(xs)
	defer func() {
		for i := 0; i < limit; i++ {
			println(i)
		}
	}()
	limit = 0
}

// Started runs a call on a goroutine of its own.
func Started(wg *sync.WaitGroup, xs []int) {
	go work(wg, xs)
}

func work(wg *sync.WaitGroup, xs []int) {
	defer wg.Done()
}
