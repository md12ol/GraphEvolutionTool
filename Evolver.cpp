#include "Evolver.h"

template<class representation>
Evolver<representation>::Evolver(char pathToConfigFile[]){

}

template<class representation>
Evolver<representation>::Evolver(const string& pathToConfigFile) : Evolver(*pathToConfigFile.c_str()) {}
