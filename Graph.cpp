#include "Graph.h"

/**
 * This constructor initializes a Graph object with numNodes number of nodes and sets the flags indicating if the Graph
 * is weighted, directed, and what representation to use to store the edges.
 *
 * @param numNodes
 * @param weightedGraph
 * @param multiGraph
 * @param directedGraph
 * @param adjListRep        // Should connectivity be stored as lists or should a matrix be used?
 */
Graph::Graph(int numNodes, bool weightedGraph, bool multiGraph, bool directedGraph, bool adjListRep) {

}

/**
 * This constructor generates a Graph object specified using the config file with path pathToConfigFile.
 * First, we ensure that a file actually exists at this path.  If it does not then an error message is displayed.
 * Second, the file is read using the readConfigFile(...) method which will ensure that the file is formatted properly
 * and initialize the instance variables of the object.  If this completes properly, the readConfigFile(...) method
 * will return an empty string (""), otherwise a description of the first error encountered will be returned.  If there
 * is an error this message is sent to the user.
 *
 * @param pathToConfigFile
 */
Graph::Graph(char pathToConfigFile[]) {
    fstream configStream;
    configStream.open(pathToConfigFile, ios::in);
    if (configStream.fail()){
        cout<<"FAIL!";
        return;
    }
    readConfigFile(configStream);
}

/**
 * This constructor delegates to the Graph constructor expecting a char array containing the path to the config file.
 * This allows the user to provide the path as a string without the program crashing.
 * The functionality is identical to that constructor.
 *
 * @param pathToConfigFile
 */
Graph::Graph(const string& pathToConfigFile) : Graph(*pathToConfigFile.c_str()) {}

/**
 * This method reads the config file contained in the filestream configFile to initialize this Graph object.  This
 * includes checking for any errors that may be encountered and initializing all instance variables of the object.
 * If an error is found then an error message is generated and returned describing the error encountered.  If the
 * initialization of the object completes successfully then an empty string is returned.
 *
 * @param configFile
 * @return error message or ""
 */
string Graph::readConfigFile(fstream &configFile) {
    return {};
}
