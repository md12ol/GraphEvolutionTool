#ifndef GRAPH_H
#define GRAPH_H

#include <iostream>
#include <vector>
#include <fstream>
#include <sstream>
#include <cmath>

using namespace std;

class Graph {
public:
    Graph();
    explicit Graph(int nn);

    int fill(string fname);
    int fill(vector<int> &weights, bool diag);
    vector<int> SIR(double alpha, int p0);
    void print(ostream &out);
    vector<int> weightHist();
    int hammy_distance(Graph &other);

protected:
    static int infect(int nin, double alpha);
    int numNodes;
    int numEdges;
    int totWeight;
    int maxWeight;
    vector<vector<int> > adjM;
};


#endif
