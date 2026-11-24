// Loops over a set the function can't change.
#include <set>

// Scans a set through a const reference.
int members(const std::set<int> &seen) {
  int total = 0;
  for (int item : seen) {
    total += item;
  }
  return total;
}
