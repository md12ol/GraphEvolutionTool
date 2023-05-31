#include <string>
#include <fstream>
#include <iostream>
#include <vector>

using namespace std;

class Graph {
public:
    Graph();
    explicit Graph(int numNodes, bool weightedGraph = false, bool multiGraph = false, bool directedGraph = false, bool adjListRep = false);
    explicit Graph(char pathToConfigFile[]);
    explicit Graph(const string& pathToConfigFile);

    int printGraph(fstream &printTo);

private:
    string readConfigFile(fstream &configFile);
    int addEdge(int from, int to, float weight = 1.0);
    int removeEdge(int from, int to, float weight = 1.0);
    float edgeWeight(int from, int to);

    int numNodes;
    bool weightedGraph;
    bool multiGraph;
    bool directedGraph;
    bool adjListRep;

    vector<vector<int>> graphAdjacency;
    vector<vector<float>> edgeWeights;
};