#include "SDA.h"

SDA::SDA(int numStates, int numChars, int maxRespLen, int outputLen, int initState, bool verbose) {
    initChar = -1;
    this->numStates = numStates;
    this->initState = initState;
    this->numChars = numChars;
    this->maxRespLen = maxRespLen;
    this->outputLen = outputLen;
    this->verbose = verbose;

    transitions.reserve(numStates);
    for (vector<int> v: transitions) {
        v.reserve(numChars);
    }

    responses.reserve(numStates);
    for (vector<vector<int> > vec1: responses) {
        vec1.reserve(numChars);
        for (vector<int> vec2: vec1) {
            vec1.reserve(maxRespLen);
        }
    }
    initialize();
    if (verbose) cout << "SDA Made with " << numStates << " numStates." << endl;
}

SDA::SDA() : SDA(10, 2, 2, 1000) {}

SDA::SDA(SDA &other) {
    copy(other);
}

SDA::~SDA() = default;

int SDA::initialize() {// Same as the create method the name was just changes
    initChar = (int) lrand48() % numChars;

    vector<int> oneState;
    oneState.reserve(numChars);
    for (int state = 0; state < numStates; ++state) {
        oneState.clear();
        for (int val = 0; val < numChars; ++val) {
            oneState.push_back((int) lrand48() % numStates);
        }
        transitions.push_back(oneState);
    }

    vector<int> oneResponse;
    oneResponse.reserve(maxRespLen);
    vector<vector<int>> oneStateResps;
    oneStateResps.reserve(numChars);
    int respSize;
    for (int state = 0; state < numStates; ++state) {
        oneStateResps.clear();
        for (int trans = 0; trans < numChars; ++trans) {
            oneResponse.clear();
            respSize = (int) lrand48() % maxRespLen + 1;
            for (int val = 0; val < respSize; ++val) {
                oneResponse.push_back((int) lrand48() % numChars);
            }
            oneStateResps.push_back(oneResponse);
        }
        responses.push_back(oneStateResps);
    }
    return 0;
}

int SDA::randomize() {
    if (initChar < 0) {
        cout << "Error in SDA Class: randomize(): this SDA has not been initialized.";
        return -1;
    }

    initChar = (int) lrand48() % numChars;

    vector<int> oneResponse;
    oneResponse.reserve(maxRespLen);
    int respLen;
    for (int state = 0; state < numStates; ++state) {
        for (int trans = 0; trans < numChars; ++trans) {
            transitions[state][trans] = (int) lrand48() % numStates;
            oneResponse.clear();
            respLen = (int) lrand48() % maxRespLen + 1;
            for (int val = 0; val < respLen; ++val) {
                oneResponse.push_back((int) lrand48() % numChars);
            }
            responses[state][trans] = oneResponse;
        }
    }
    if (verbose) cout << "SDA Randomized." << endl;
    return 0;
}

int SDA::copy(SDA &other) {
    if (initChar < 0) {
        cout << "Error in SDA Class: copy(...): this SDA has not been initialized.";
        return -1;
    }
    if (other.initChar < 0) {
        cout << "Error in SDA Class: copy(...): other SDA has not been initialized.";
        return -1;
    }

    initChar = other.initChar;
    numStates = other.numStates;
    initState = other.initState;
    numChars = other.numChars;
    maxRespLen = other.maxRespLen;
    outputLen = other.outputLen;
    verbose = other.verbose;

    transitions = other.transitions;
    responses = other.responses;
    if (verbose) cout << "SDA Copied." << endl;
    return 0;
}

int SDA::crossover(SDA &other) {
    if (initChar < 0) {
        cout << "Error in SDA Class: twoPtCrossover(...): this SDA has not been initialized.";
        return -1;
    }
    if (other.initChar < 0) {
        cout << "Error in SDA Class: twoPtCrossover(...): other SDA has not been initialized.";
        return -1;
    }
    if (numStates != other.numStates) {
        cout << "Error in SDA Class: twoPtCrossover(...): the two SDAs have a different numStates.";
        return 1;
    }
    if (numChars != other.numChars) {
        cout << "Error in SDA Class: twoPtCrossover(...): the two SDAs have a different numChars.";
        return 1;
    }
    if (maxRespLen != other.maxRespLen) {
        cout << "Error in SDA Class: twoPtCrossover(...): the two SDAs have a different maxRespLen.";
        return 1;
    }

    int cp1, cp2;
    int swapInt;
    vector<int> swapVec;
    swapVec.reserve(numChars);

    do {
        cp1 = (int) lrand48() % numStates;
        cp2 = (int) lrand48() % numStates;
        if (cp1 > cp2) {
            swapInt = cp1;
            cp1 = cp2;
            cp2 = swapInt;
        }
    } while (cp1 == cp2);

    if (cp1 == 0) {
        swapInt = initChar;
        initChar = other.initChar;
        other.initChar = swapInt;
    }

    for (int s = cp1; s < cp2; s++) {
        swapVec = transitions.at(s);
        transitions.at(s) = other.transitions.at(s);
        other.transitions.at(s) = swapVec;
        swapVec = responses.at(s).at(0);
        responses.at(s).at(0) = other.responses.at(s).at(0);
        other.responses.at(s).at(0) = swapVec;
        swapVec = responses.at(s).at(1);
        responses.at(s).at(1) = other.responses.at(s).at(1);
        other.responses.at(s).at(1) = swapVec;
    }
    return 0;
}

int SDA::mutate(int numMuts) {
    if (initChar < 0) {
        cout << "Error in SDA Class: mutate(...): this SDA has not been initialized.";
        return -1;
    }

    int mutPt;
    vector<int> oneResponse;
    int respSize;

    for (int mut = 0; mut < numMuts; ++mut) {
        mutPt = (int) lrand48() % (2 * numStates + 1);

        if (mutPt == 0) {
            initChar = (int) lrand48() % numChars;
            return 0;
        }
        mutPt = (mutPt - 1) / 2;
        int transNum = (int) lrand48() % numChars;
        if ((int) lrand48() % 2 == 0) { // Mutate transition
            transitions.at(mutPt).at(transNum) = (int) lrand48() % numStates;
        } else { // Mutate response
            oneResponse.clear();
            respSize = (int) lrand48() % 2 + 1;
            for (int i = 0; i < respSize; ++i) {
                oneResponse.push_back((int) lrand48() % numChars);
            }
            responses.at(mutPt).at(transNum) = oneResponse;
        }
    }
    return 0;
}

int SDA::express(Graph &G) {
    return 0;
}

int SDA::print(ostream &outStrm = cout) {
    if (initChar < 0) {
        cout << "Error in SDA Class: printSDA(...): this SDA has not been initialized.";
        return -1;
    }

    outStrm << initState << " <- " << initChar << endl;
    for (int state = 0; state < numStates; ++state) {
        for (int t = 0; t < numChars; ++t) {
            outStrm << state << " + " << t << " -> " << transitions.at(state).at(t) << " [";
            for (int v: responses.at(state).at(t)) {
                outStrm << " " << v;
            }
            outStrm << " ]" << endl;
        }
    }
    if (verbose) cout << "SDA Printed." << endl;
    return 0;
}
