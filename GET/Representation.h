#include <iostream>
#include "Graph.h"

#ifndef GRAPHEVOLUTIONTOOL_REPRESENTATION_H
#define GRAPHEVOLUTIONTOOL_REPRESENTATION_H

class Representation {
public:
    Representation();
    Representation(Representation &other);
    ~Representation();

    int randomize();
    int copy(Representation &other);
    int crossover(Representation &other);
    int mutate(int numMuts);
    int express(Graph &G);
    int print(ostream &to);

private:
    int initialize();
};

#endif //GRAPHEVOLUTIONTOOL_REPRESENTATION_H
