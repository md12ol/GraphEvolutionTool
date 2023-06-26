#include "Graph.h"

Graph::Graph() {
    numEdges = 0;
    graphAdjacency.reserve(NUM_NODES);
    vector<int> oneNodesAdj;
    if (MULTI_GRAPH && WEIGHTED_GRAPH){
        oneNodesAdj.reserve(NUM_NODES * MAX_WEIGHT);
    } else {

    }

    if (!ADJ_LIST_REP){
        for (int i = 0; i < NUM_NODES; ++i) {
            oneNodesAdj.push_back(0);
        }
    } else {
        oneNodesAdj = {};
    }
    for (int i = 0; i < NUM_NODES; ++i) {
        graphAdjacency.push_back(oneNodesAdj);
    }



}
