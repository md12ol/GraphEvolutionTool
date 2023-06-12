#include <iostream>
#include <vector>

#include "Graph.h"

using namespace std;

class EEOPS {
public:
    EEOPS();
    EEOPS(int numOps);
    EEOPS(EEOPS &other);
    ~EEOPS();

    int randomize();
    int copy(EEOPS &other);
    int crossover(EEOPS &other);
    int mutate(int numMuts);
    int express(Graph &G);
    int print(ostream &to);

private:
    int initialize();

    int add(int from, int to);
    int localAdd(int from, int to, int neighbour);
    int del(int from, int to);
    int localDel(int from, int to, int neighbour);
    int toggle(int from, int to);
    int localToggle(int from, int to, int neighbour);
    int swap(int from, int to, int neighbour);
    int hop(int from, int to, int neighbour);

    int generateGene();

    int numGenes; // Length of chromosome
    vector<int> genes;
};
