// While loops whose condition calls something before it compares.

int count(unsigned value) { return static_cast<int>(value) + 1; }

// Calls count before each comparison, so the loop still counts up to limit.
unsigned sequenced(unsigned limit) {
  unsigned i = 0;
  while (count(i), i < limit) {
    i++;
  }
  return i;
}
