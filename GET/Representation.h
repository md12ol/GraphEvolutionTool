#include <iostream>
#include "Graph.h"

#ifndef GRAPHEVOLUTIONTOOL_REPRESENTATION_H
#define GRAPHEVOLUTIONTOOL_REPRESENTATION_H

class Representation {
public:

    virtual int randomize() = 0;
    virtual int copy(Representation &other) = 0;
    virtual int crossover(Representation &other) = 0;
    virtual int mutate(int numMuts) = 0;
    virtual int express(Graph &G) = 0;
    virtual int print(ostream &to) = 0;

    ~Representation() = default;

protected:
    Representation() = default;
    Representation(Representation &other) = default;

private:
    virtual int initialize() = 0;
};

#endif //GRAPHEVOLUTIONTOOL_REPRESENTATION_H
