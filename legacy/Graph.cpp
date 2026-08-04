#include "Graph.h"
#include "../BitSprayer/Bitsprayer.h"

Graph::Graph() {
    numNodes = 0;
    numEdges = 0;
    totWeight = 0;
}

Graph::Graph(int nn) {
    numNodes = nn;
    numEdges = 0;
    totWeight = 0;

    adjM.reserve(nn);
    vector<int> row(nn);
    for (int i = 0; i < nn; ++i) {
        adjM.push_back(row);
    }
}

int Graph::fill(const string fname) {
    numNodes = 0;
    numEdges = 0;
    totWeight = 0;

    ifstream infile(fname);
    string line;
    infile >> numNodes;
    getline(infile, line);
    getline(infile, line);

    adjM.reserve(numNodes);
    vector<int> row(numNodes);
    for (int i = 0; i < numNodes; ++i) {
        adjM.push_back(row);
    }

    int from = 0;
    while (getline(infile, line)) {
        stringstream ss(line);

        int to;
        while (ss >> to) {
            if (from < to) {
                if (adjM[from][to] == 0) {
                    numEdges++;
                }
                adjM[from][to]++;
                adjM[to][from]++;
                totWeight++;
            }
        }
        from++;
    }
    return 0;
}

void Graph::print(ostream &out) {
    out << "Nodes: " << numNodes << endl;
    out << "Edges: " << numEdges << endl;
    out << "Tot Weight: " << totWeight << endl;
    out << "W Hist: ";
    for (int v: weightHist()) {
        out << v << " ";
    }
    out << endl;
    for (int row = 0; row < numNodes; ++row) {
        for (int col = 0; col < numNodes; ++col) {
            if (adjM[row][col] > 0) {
//            out << adjM[row][col] << " ";
//                out << col << "[" << adjM[row][col] << "] ";
                for (int i = 0; i < adjM[row][col]; ++i) {
                    out << col << " ";
                }
            }
        }
        out << endl;
    }
}

vector<int> Graph::weightHist() {
    vector<int> rtn;
    rtn.reserve(6);
    for (int i = 0; i < 6; ++i) {
        rtn.push_back(0);
    }
    for (int node = 0; node < numNodes; ++node) {
        for (int to = node + 1; to != node && to < numNodes; to++) {
            rtn[adjM[node][to]]++;
        }
    }
    return rtn;
}

int quadForm(int A, int B, int C) {
    return ((-1) * B + (int) sqrt(B * B - 4 * A * C)) / (2 * A);
}

int Graph::fill(vector<int> &weights, bool diag) {
    int nn = quadForm(1, -1, (-1) * (int) weights.size() * 2);
    if (nn != numNodes) {
        cout << "ERROR!  numNodes updated incorrectly!!" << endl;
        numNodes = nn;
    }

    int idx = 0;
    if (diag) {
        int col;
        for (int iter = 1; iter < numNodes; ++iter) {
            for (int row = 0; row < numNodes - iter; ++row) {
                col = row + iter;
                adjM[row][col] = weights[idx];
                adjM[col][row] = weights[idx];
                if (adjM[row][col] > 0) {
                    numEdges += 1;
                }
                totWeight += weights[idx];
                idx += 1;
            }
        }
        if (idx != weights.size()) {
            cout << "ERROR! Diag fill not working!" << endl;
        }
    } else {
        for (int row = 0; row < numNodes; row++) {
            for (int col = row + 1; col < numNodes; col++) {
                adjM[row][col] = weights[idx];
                adjM[col][row] = weights[idx];
                if (adjM[row][col] > 0) {
                    numEdges += 1;
                }
                totWeight += weights[idx];
                idx += 1;
            }
        }
    }
    return 0;
}

int Graph::infect(int nin, double alpha) {
    double beta;

    beta = 1 - exp(nin * log(1 - alpha));
    if (drand48() < beta) return (1);
    return (0);
}

vector<int> Graph::SIR(double alpha, int p0) {
    int curI, len;
    vector<int> nin(numNodes);
    vector<int> state(numNodes);
    vector<int> profile;
    profile.reserve(numNodes);

    for (int i = 0; i < numNodes; i++) {
        state[i] = 0;
    }
    len = 0;
    state[p0] = 1;
    curI = 1;
    profile.push_back(curI);

    while (curI > 0) {
        for (int i = 0; i < numNodes; ++i) {
            nin[i] = 0;
        }
        for (int node = 0; node < numNodes; ++node) {
            if (state[node] == 1) {
                for (int neigh = 0; neigh < numNodes; neigh++) {
                    if (neigh != node && adjM[node][neigh] > 0) {
                        nin[neigh] += adjM[node][neigh];
                    }
                }
            }
        }
        for (int node = 0; node < numNodes; node++) {
            if (state[node] == 0 && nin[node] > 0) {
                if (infect(nin[node], alpha) == 1) {
                    state[node] = 3;
                }
            }
        }
        curI = 0;
        for (int node = 0; node < numNodes; ++node) {
            switch (state[node]) {
                case 0:
                    break;
                case 1:
                    state[node] = 2;
                    break;
                case 2:
                    break;
                case 3:
                    state[node] = 1;
                    curI += 1;
                    break;
            }
        }
        len += 1;
        profile.push_back(curI);
    }
    return profile;
}

int Graph::hammy_distance(Graph &other){
    int cost = 0;
    for (int row = 0; row < other.numNodes; ++row) {
        for (int col = row+1; col < other.numNodes; ++col) {
            int curCount = adjM[row][col];
            int dubCount = other.adjM[row][col];
            if (curCount!=dubCount){
                if (dubCount == 0){
                    cost += 5;
                } else if (curCount == 0){
                    cost += 5;
                } else{
                    cost += abs(curCount-dubCount);
                }
            }
        }
    }
    return cost;
}

//int main(){
//    Bitsprayer b;
//    b.create(128);
//    b.randomize();
//    Graph G(128);
//    int size = 128 * (128 - 1) / 2;
//    vector<int> weights = b.getBitsVec(size);
//    b.printBitsVec(size, cout);
//    G.fill(weights);
//    G.print(cout);
//}