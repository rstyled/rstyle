// Hands callables to the standard library, and calls one where it's written.
#include <mutex>
#include <optional>
#include <vector>

int count(int value) { return value + 1; }

// Scans ys for each of xs, and hands transform a lambda to run at most once.
int mapped(const std::vector<int> &xs, const std::vector<int> &ys, std::optional<int> held) {
  int total = held.transform([](int value) { return value + 1; }).value_or(0);
  for (int x : xs) {
    for (int y : ys) {
      total += x * y;
    }
  }
  return total;
}

// Hands and_then a scan of xs, and or_else a value for when there is none.
int chained(const std::vector<int> &xs, std::optional<int> held) {
  auto next = held.and_then([&xs](int value) -> std::optional<int> {
    int sum = value;
    for (int x : xs) {
      sum += count(x);
    }
    return sum;
  });
  return next.or_else([] { return std::optional<int>(0); }).value_or(1);
}

// Hands call_once a scan of xs, after the flag.
void once(const std::vector<int> &xs, std::once_flag &flag) {
  std::call_once(flag, [&xs] {
    for (int x : xs) {
      count(x);
    }
  });
}

// Hands transform a function by its name.
int named(std::optional<int> held) { return held.transform(count).value_or(0); }

// Hands transform a callable only its caller knows.
int value(std::optional<int> held, int (*callable)(int)) { return held.transform(callable).value_or(0); }

// Hands call_once a lambda that writes what it captures.
int written(const std::vector<int> &xs, std::once_flag &flag) {
  int total = 0;
  std::call_once(flag, [&] {
    for (int x : xs) {
      total += x;
    }
  });
  return total;
}

// Calls a lambda where it's written.
int immediate(const std::vector<int> &xs) {
  return [&xs] {
    int sum = 0;
    for (int x : xs) {
      sum += count(x);
    }
    return sum;
  }();
}
