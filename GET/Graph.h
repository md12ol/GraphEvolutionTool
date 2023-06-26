#include <string>
#include <fstream>
#include <iostream>
#include <vector>
#include "config.h"

using namespace std;

class Graph {
public:
    Graph();

    int printGraph(fstream &printTo);

private:
    int addEdge(int from, int to, float weight = 1.0);
    int removeEdge(int from, int to, float weight = 1.0);
    float edgeWeight(int from, int to);

    int numEdges;

    vector<vector<int>> graphAdjacency;
    vector<vector<float>> edgeWeights;
};

/**
 * Some notes on the graph class for us.  A graph can be:
 * Directed or Undirected
 * Weighted or Unweighted
 * A Multi-Graph or Non-Multi-Graph
 * Represented on an Adjacency List or Adjacency Matrix
 * This presents 16 possible different types of graphs.  But we can represent all 16 with the two vectors:
 * graphAdjacency: a vector of integer containing vectors.  Each vector stores the adjacency for one node.
 * edgeWeights: a vector of float containing vectors.  Each vector stores the weights for one node's edges.
 *
 * Adjacency List:
 *  Each vector in graphAdjacency starts as an empty vector.
 *      Weighted Directed Multi-Graph:
 *          Each entry in each graphAdjacency vector corresponds to one edge from this node to the node with the value
 *              in the vector.  These values can repeat.
 *          Each entry in each edgeWeights vector corresponds to the weight of the edge with the same index in the
 *              corresponding graphAdjacency vector.
 *          For example, say we have a graph with three nodes.
 *              graphAdjacency =    {{1, 1, 2},
 *                                  {0, 2},
 *                                  {0, 1, 1}}
 *              edgeWeights =       {{0.5, 3.2, 9},
 *                                  {3, 4},
 *                                  {1, 2, 3}}
 *              Then, Node 0 has two edges to Node 1 (with weights 0.5 and 3.2) and one edge to Node 2 (weight 9).
 *                  Node 1 has one edge to Node 0 (weight 3) and one to Node 2 (weight 4).  And Node 2 has one edge
 *                  to Node 0 (weight 1) and two edges to Node 1 (weights 2 and 3).
 *      Weighted (Un)Directed Multi-Graph:
 *          Mostly the same as Weighted Directed Multi-Graph, except the edges and weights are symmetric.
 *          For example, say we have a graph with three nodes.
 *              graphAdjacency =    {{1, 1, 2},
 *                                  {0, 0, 2, 2},
 *                                  {0, 1, 1}}
 *              edgeWeights =       {{0.5, 3.2, 9},
 *                                  {0.5, 3.2, 2, 3},
 *                                  {9, 2, 3}}
 *              Then, Node 0 has two edges to Node 1 (with weights 0.5 and 3.2) and one edge to Node 2 (weight 9).
 *                  Node 1 has two edges to Node 0 (weights 0.5 and 3.2) and two edges to Node 2 (weights 2 and 3).
 *                  And Node 2 has one edge to Node 0 (weight 9) and two edges to Node 1 (weights 2 and 3).
 *      (Un)Weighted Directed Multi-Graph:
 *          Similar to Weighted Directed Multi-Graph, except there is no need to use the edgeWeights vector as each
 *              edge has a weight one 1.
 *          For example, say we have a graph with three nodes.
 *              graphAdjacency =    {{1, 1, 2},
 *                                  {0, 2},
 *                                  {0, 1, 1}}
 *              Then, Node 0 has two edges to Node 1 and one edge to Node 2.  Node 1 has one edge to Node 0 and one
 *                  edge to Node 2.  And Node 2 has one edge to Node 0 and two edges to Node 1.
 *      (Un)Weighted (Un)Directed Multi-Graph:
 *          Similar to (Un)Weighted Directed Multi-Graph, except the edges are symmetric.
 *          For example, say we have a graph with three nodes.
 *              graphAdjacency =    {{1, 1, 2},
 *                                  {0, 0, 2, 2},
 *                                  {0, 1, 1}}
 *              Then, Node 0 has two edges to Node 1 and one edge to Node 2.  Node 1 has two edges to Node 0 and two
 *                  edges to Node 2.  And Node 2 has one edge to Node 0 and two edges to Node 1.
 *      Weighted Directed (Non-)Multi-Graph:
 *          Similar to Weighted Directed Multi-Graph, except the values in graphAdjacency cannot repeat.
 *          For example, say we have a graph with three nodes.
 *              graphAdjacency =    {{1, 2},
 *                                  {2},
 *                                  {0}}
 *              edgeWeights =       {{4.3, 8},
 *                                  {3},
 *                                  {6.3}}
 *              Then, Node 0 has an edge to Node 1 (with weight 4.3) and an edge to Node 2 (weight 8).  Node 1 has an
 *                  edge to Node 2 (weight 3).  And Node 2 has an edge to Node 0 (weight 6.3).
 *      Weighted (Un)Directed (Non-)Multi-Graph:
 *          Similar to Weighted Directed (Non-)Multi-Graph, except the edges and weights are symmetric.
 *          For example, say we have a graph with three nodes.
 *              graphAdjacency =    {{1, 2},
 *                                  {0, 2},
 *                                  {0, 1}}
 *              edgeWeights =       {{4.3, 8},
 *                                  {4.3, 3},
 *                                  {8, 3}}
 *              Then, Node 0 has an edge to Node 1 (with weight 4.3) and an edge to Node 2 (weight 8).  Node 1 has an
 *                  edge to Node 0 (weight 4.3) and an edge to Node 2 (weight 3).  And Node 2 has an edge to Node 0
 *                  (weight 8) and an edge to Node 1 (weight 3).
 *      (Un)Weighted Directed (Non-)Multi-Graph:
 *          Similar to Weighted Directed (Non-)Multi-Graph, except there is no need to use the edgeWeights vector as
 *              each edge has a weight of 1.
 *          For example, say we have a graph with three nodes.
 *              graphAdjacency =    {{1, 2},
 *                                  {2},
 *                                  {0}}
 *              Then, Node 0 has an edge to Node 1 and an edge to Node 2.  Node 1 has an edge to Node 2.  And Node 2
 *                  has an edge to Node 0.
 *      (Un)Weighted (Un)Directed (Non-)Multi-Graph:
 *          Similar to (Un)Weighted Directed (Non-)Multi-Graph, except the edges are symmetric.
 *          For example, say we have a graph with three nodes.
 *              graphAdjacency =    {{1, 2},
 *                                  {0, 2},
 *                                  {0, 1}}
 *              Then, Node 0 has an edge to Node 1 and an edge to Node 2.  Node 1 has an edge Node 0 and one to Node 2.
 *                  And Node 2 has an edge to Node 0 and an edge to Node 1.
 *
 * Adjacency Matrix:
 *  Each vector in graphAdjacency starts as vector of zeros.
 *  The functionality is unchanged if Directed or Undirected.
*/