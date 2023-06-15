#include "Graph.h"

int main(int argc, char *argv[]){

    char pathToConfigArr[] = "./thing.dat";
    string pathToConfigStr = "./thing.dat";

    Graph G = Graph(100);
    Graph G1 = Graph(pathToConfigArr);
    Graph G2 = Graph(pathToConfigStr);

}