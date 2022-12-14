opy src\* .\TinyInst\

:: this will give you new litecov.exe to test on with Vector Coverage instead of list coverage 
copy test\tinyinst-coverage.cpp TinyInst\tinyinst-coverage.cpp

cxxbridge ./src/tinyinst.rs -o ./TinyInst/bridge.cc
cxxbridge ./src/tinyinst.rs --header -o ./TinyInst/bridge.h
cxxbridge --header -o ./TinyInst/cxx.h
cmake --build build