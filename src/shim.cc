#include "shim.h"

std::unique_ptr<Coverage> coverage_new()
{
  return std::make_unique<Coverage>();
}

void get_coverage_map(uint8_t *bitmap, size_t map_size, Coverage &newcoverage)
{
  for (auto iter = newcoverage.begin(); iter != newcoverage.end(); iter++)
  {
    for (auto &offset : iter->offsets)
    {
      if (offset < map_size)
      {
        bitmap[offset] = 1;
      }
    }
  }
}

// tinyinstinstrumentation
std::unique_ptr<TinyInstInstrumentation> tinyinstinstrumentation_new()
{
  return std::make_unique<TinyInstInstrumentation>();
}
