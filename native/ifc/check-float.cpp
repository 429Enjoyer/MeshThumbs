// Regression check linked against the actual patched IfcParse library.
#include <cmath>
#include <iostream>

bool ParseFloat(const char *, double &);

int main() {
  struct Case { const char *token; double expected; };
  for (auto item : {Case{"0.0001", 0.0001}, Case{"1.", 1.},
                    Case{"-2.5E+3", -2500.}, Case{"1e-5", 1e-5},
                    Case{"0.0", 0.}, Case{"+3.25", 3.25}}) {
    double value = 0.;
    if (!ParseFloat(item.token, value) || std::abs(value - item.expected) > 1e-12) {
      std::cerr << "Valid IFC real rejected: " << item.token << '\n';
      return 1;
    }
  }
  for (auto token : {"1.2x", "1e", "#8", "UNKNOWN", "1e9999"}) {
    double value = 0.;
    if (ParseFloat(token, value)) {
      std::cerr << "Invalid IFC real accepted: " << token << '\n';
      return 1;
    }
  }
  std::cout << "IFC real parser: 11 cases passed\n";
}
