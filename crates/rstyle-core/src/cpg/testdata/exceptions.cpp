// Throws and catches around scans of vectors the functions only read.
#include <stdexcept>
#include <vector>

// Throws inside a scan, which leaves it early.
void checked(const std::vector<int> &xs) {
  for (int x : xs) {
    if (x < 0) {
      throw std::invalid_argument("negative");
    }
  }
}

// Catches what each item's check throws by scanning another vector.
int handled(const std::vector<int> &xs, const std::vector<int> &ys) {
  int total = 0;
  for (int x : xs) {
    try {
      if (x == 0) {
        throw 1;
      }
      total += 100 / x;
    } catch (int) {
      for (int y : ys) {
        total += y;
      }
    }
  }
  return total;
}

// A limit set partway through a body that threw is unknown to the loop after it.
int interrupted(unsigned n) {
  unsigned limit = 0;
  unsigned i = 0;
  try {
    limit = n;
    if (n == 0) {
      throw 1;
    }
    limit = 0;
  } catch (...) {
  }
  while (i < limit) {
    i += 1;
  }
  return 0;
}

// Catches inside a lambda called where it's written, which returns from the lambda.
int inlined(const std::vector<int> &xs) {
  [] {
    try {
      for (int i = 0; i < 3; i++) {
        if (i == 2) {
          throw i;
        }
      }
    } catch (...) {
      return;
    }
  }();
  return 0;
}
