#include <iostream>
#include <vector>

#include "Graph.h"

using namespace std;

class SDA {
public:
    SDA();
    explicit SDA(int numStates, int numChars, int maxRespLen, int outputLen, int initState = 0, bool verbose = false);
    SDA(SDA &other);
    ~SDA();

    int randomize();
    int copy(SDA &other);
    int crossover(SDA &other);
    int mutate(int numMuts);
    int express(Graph &G);
    int print(ostream &outStrm);

private:
    int initialize();

    int initChar{};
    int numStates{};
    int initState{};
    int curState{};
    int numChars{};
    int maxRespLen{};
    int outputLen{};
    bool verbose{};

    vector<vector<int> > transitions;
    vector<vector<vector<int> > > responses;
};
