// Package errs is a fixture for where the errors a function returns come from.
package errs

import (
	"errors"
	"fmt"
	"strconv"
)

// ErrClosed is a sentinel error.
var ErrClosed = errors.New("closed")

func open(name string) error {
	if name == "" {
		return errors.New("no name")
	}
	if name == "closed" {
		return ErrClosed
	}
	return nil
}

// Load wraps what open returns.
func Load(name string) error {
	if err := open(name); err != nil {
		return fmt.Errorf("load %s: %w", name, err)
	}
	return nil
}

// Last returns only what it assigned last.
func Last() error {
	err := errors.New("first")
	err = errors.New("second")
	return err
}

// Each returns what a function value does, and skips a step that isn't there.
func Each(steps []func() error) error {
	for _, step := range steps {
		if step == nil {
			continue
		}
		if err := step(); err != nil {
			return err
		}
	}
	return nil
}

// Parse returns a library call's error beside its value.
func Parse(text string) (int, error) {
	n, err := strconv.Atoi(text)
	return n, err
}

// Later's closure returns an error of its own, which isn't Later's.
func Later(run func(func() error)) error {
	run(func() error {
		return errors.New("inside")
	})
	return errors.New("outside")
}

// Retry wraps the error it had before, each time round.
func Retry(times int) error {
	err := errors.New("never tried")
	for i := 0; i < times; i++ {
		err = fmt.Errorf("try %d: %w", i, err)
	}
	return err
}

// First returns one of the errors it's given.
func First(errs []error) error {
	return errs[0]
}

type closedError struct{}

func (closedError) Error() string { return "closed" }

func newClosed() *closedError { return &closedError{} }

// Shut returns an error of its own type, not error.
func Shut() error {
	return newClosed()
}
