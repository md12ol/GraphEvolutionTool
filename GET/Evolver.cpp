#include "Evolver.h"

template<class representation>
Evolver<representation>::Evolver(char pathToConfigFile[]){

    // Initialize the population
    // Calculate fitness of each member of the population
    // Generate the first repo

}

template<class representation>
Evolver<representation>::Evolver(const string& pathToConfigFile) : Evolver(*pathToConfigFile.c_str()) {}
