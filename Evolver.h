#include <string>
#include <fstream>
#include <iostream>
#include <vector>

#include "SDA.h"
#include "EEOPS.h"
#include "Graph.h"

using namespace std;

template<class representation>
class Evolver {
public:
    explicit Evolver(char pathToConfigFile[]);
    explicit Evolver(const string &pathToConfigFile);

    int evolve(bool verbose = true);
    float fitness(Graph &G);

private:
    bool biggerBetter;
    int popSize;
    int numGenerations;
    int tournSize;
    float crossoverProb;
    float mutationProb;
    pair<int, int> numMuts;

    vector<representation> population;
    vector<float> fitnessVals;
};
