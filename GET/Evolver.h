#include <string>
#include <fstream>
#include <iostream>
#include <vector>

#include "SDA.h"
#include "GET/EEOPS.h"
#include "Graph.h"

using namespace std;

template<class representation>
class Evolver {
public:
    explicit Evolver();

private:
    int popSize;
    int numGenerations;
    int tournSize;
    float crossoverProb;
    float mutationProb;
    pair<int, int> numMuts;

    vector<representation> population;
    vector<float> fitnessVals;
};

