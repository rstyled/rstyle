// Package errs is a fixture for where the errors a function returns come from.
package errs

import (
	"errors"
	"fmt"
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

// Each returns what a function value does.
func Each(steps []func() error) error {
	for _, step := range steps {
		if err := step(); err != nil {
			return err
		}
	}
	return nil
}
