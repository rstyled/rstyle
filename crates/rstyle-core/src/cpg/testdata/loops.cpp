// Loops whose guard comes after their body, or that have none.
#include <vector>

int count(int value) { return value + 1; }

// Runs its body once, then again while the counter is under the limit.
unsigned repeated(unsigned limit) {
  unsigned i = 0;
  do {
    count(i);
    i++;
  } while (i < limit);
  return i;
}

// Leaves a loop with no guard by breaking out of it.
unsigned forever(unsigned limit) {
  unsigned i = 0;
  for (;;) {
    if (i >= limit) {
      break;
    }
    i++;
  }
  return i;
}

// Skips the rest of a pass with continue, which goes to the guard.
unsigned skipping(const std::vector<int> &xs) {
  unsigned i = 0;
  unsigned odd = 0;
  do {
    i++;
    if (i % 2 == 0) {
      continue;
    }
    odd++;
  } while (i < xs.size());
  return odd;
}

#define CHECKED(value) \
  do {                 \
    count(value);      \
  } while (0)
#define LOGGED(value) \
  do {                \
    count(value);     \
  } while ((void)0, false && (value) > 0)
#define CAST(value) \
  do {              \
    count(value);   \
  } while (static_cast<bool>(0))

// Three macros' do statements, whose tests are false, each run their body once per item.
unsigned macros(const std::vector<int> &xs) {
  unsigned total = 0;
  for (int x : xs) {
    CHECKED(x);
    LOGGED(x);
    CAST(x);
    total++;
  }
  return total;
}

// A do statement that tests false runs once, and continue goes to the test.
void once(unsigned limit) {
  do {
    if (limit == 0) {
      continue;
    }
    count(1);
  } while (false);
}

// A do statement whose test always holds leaves only by breaking out of it.
unsigned spinning(unsigned limit) {
  unsigned i = 0;
  do {
    if (i >= limit) {
      break;
    }
    i++;
  } while (true);
  return i;
}
