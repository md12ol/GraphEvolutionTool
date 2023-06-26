#include "Evolver.h"
#include "config.h"

int tournsize = 7;

/**This class performs the evolutionary functions on the population, such as two-point crossover, mutations and
 * tournament selections.
 * The fitness of each individual member of the populace is determined before and after the evolutionary functions
 * are performed.
 * 
*/

/**This method initializes the population
 * 
*/

vector<SDA> initPop(){
    // Generate the Initial Population
    for (int idx = 0; idx < popsize; idx++) {// generate a population based on the populaiton size defined
         dead[idx] = false;// initialize ll members of the population to be alive
         SDAPop[idx].fillOutput(SDAOutput);
    //     while (necroticFilter()) {
    //         SDAPop[idx].randomize();
    //         SDAPop[idx].fillOutput(SDAOutput);
    //     }
         fits[idx] = fitness(idx, false);// get the fitness of the initial population
    }
}

/** This method performs a fitness evaluation of an SDA by examining the ...
 * 
*/

double fitness(SDA &evaluate){
    if(evaluate.transitions.size() > best){// Create function in SDA class to return number of transitions ?????
        return evaluate.transitions.size();
    }
    else return 10;
}// Fitness

vector<int> tournSelect(int size, bool decreasing) {
    vector<int> tournIdxs;// initialize vector for holding placement of population in tournament
    int idxToAdd;

    tournIdxs.reserve(size);// reserve space for tournament vector
    if (size == popsize) {// if tournament size fed into method is same as initial popsize
        for (int idx = 0; idx < size; idx++) {//push each meber of population into tournament index vector
            tournIdxs.push_back(idx);
        }
    } else {// if the popsizes are different
        do {// do this while the tournament vector size is less than the tournament size
            idxToAdd = (int) lrand48() % popsize;// randomly chooses a number within the population range
            if (count(tournIdxs.begin(), tournIdxs.end(), idxToAdd) == 0) {// check if the number is not already in tournament vector
                tournIdxs.push_back(idxToAdd);// push into tournament vector
            }
        } while (tournIdxs.size() < tournSize);
    }

    sort(tournIdxs.begin(), tournIdxs.end(), compareFitness);
    if (decreasing) {
        reverse(tournIdxs.begin(), tournIdxs.end());
    }
    return tournIdxs;
}

void matingEvent() {
    int numMuts;
    vector<int> tournIdxs;
    // Selection
    tournIdxs = tournSelect(tournSize, ctrlFitnessFctn == 1);

    // Copy the Parents -> Children
    SDAPop[tournIdxs[0]].copy(SDAPop[tournIdxs[tournSize - 2]]);
    SDAPop[tournIdxs[1]].copy(SDAPop[tournIdxs[tournSize - 1]]);

    // Crossover
    if (drand48() < crossoverRate) SDAPop[tournIdxs[0]].twoPointCrossover(SDAPop[tournIdxs[1]]);

    // Mutation
    if (drand48() < mutationRate) {
        numMuts = (int) lrand48() % maxMuts + 1;
        SDAPop[tournIdxs[0]].mutate(numMuts);
        numMuts = (int) lrand48() % maxMuts + 1;
        SDAPop[tournIdxs[1]].mutate(numMuts);
    }

    // Reset dead SDAs
    SDAPop[tournIdxs[0]].fillOutput(SDAOutput);
    //dead[tournIdxs[0]] = necroticFilter();
    SDAPop[tournIdxs[1]].fillOutput(SDAOutput);
    //dead[tournIdxs[1]] = necroticFilter();

    // if (!dead[tournIdxs[0]]) {
    //     fits[tournIdxs[0]] = fitness(tournIdxs[0], false);
    // } else {
    //     fits[tournIdxs[0]] = globalWorstFit;
    // }

    // if (!dead[tournIdxs[1]]) {
    //     fits[tournIdxs[1]] = fitness(tournIdxs[1], false);
    // } else {
    //     fits[tournIdxs[1]] = globalWorstFit;
    // }
}

template<class representation>
Evolver<representation>::Evolver() {// This method is going to be similar to the main method (with the rest of the file beinf like main.h)

    int fit[popSize];// initialize the fitness array
    SDA SDAPop[popSize];// array containing the members of the population
    boolean dead[popSize];// array holding information on whether a member of the population is dead
    int numMatings = 100;// number of mating events that will be performed on the living members of the population

    initPop();

    for(int x = 0; x < SDAPop.size(); x++){// check fitness of each SDA
        fit[x] = fitness(SDAPop[x]);// set SDA fitness in arrat at corresponding position
    }

    //Generate the report
    for(int x = 0; x < SDAPop.size(); x++){
        cout << "SDa" << x << endl;
        SDAPop[x].print(cout);
        cout << "Fitness of SDA: " << fit[x] << endl;
    }


    for(int x = 0; x < numMatings; x++){// perform the maiting events
        vector<int> tournResults;
        //perform tournament
        tournResults = tournSelect(tournSize, true);
        //perform two-point crossover on all members of the population

        //perform mutations
        //print fitness values
    }


        
    }//Evolver


    // Initialize the population
    // Calculate fitness of each member of the population
    // Generate the first report
    // Do the mating events
    // Etc.

    //if (BIGGER_BETTER){
    //    cout<<"WOO!"<<endl;
    //}

//}

/**
 * Kevin's first task:
 * 1. Write a dummy fitness method to be used on Graph's (i.e return the number of edges)
 * 
 * 2. Initialize a population of 10 representations (think of this as SDAs)
 * kev note - What is meant by representations (Where is the initial pop bing initialized)
 * 
 * 3. Print the fitness values of the population to console (before evolution)
 * 4. Run 100 mating events
 * 5. Print the fitness values of the population to console (after evolution) (each mating event)
*/