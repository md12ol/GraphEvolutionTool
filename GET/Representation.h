#include <iostream>
#include "Graph.h"

#ifndef GRAPHEVOLUTIONTOOL_REPRESENTATION_H
#define GRAPHEVOLUTIONTOOL_REPRESENTATION_H

class Representation {
public:

    virtual Representation(Representation &other) = 0;

    virtual int randomize() = 0;
    virtual int copy(Representation &other) = 0;
    virtual int crossover(Representation &other) = 0;
    virtual int mutate(int numMuts) = 0;
    virtual int express(Graph &G) = 0;
    virtual int print(ostream &to) = 0;

    ~Representation() {}

protected:
    Representation() {}

private:
    virtual int initialize() = 0;
};

#endif //GRAPHEVOLUTIONTOOL_REPRESENTATION_H
