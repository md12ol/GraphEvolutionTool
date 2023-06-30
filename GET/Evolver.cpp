#include "Evolver.h"
#include "config.h"

/**This class performs the evolutionary functions on the population, such as two-point crossover, mutations and
 * tournament selections.
 * The fitness of each individual member of the populace is determined before and after the evolutionary functions
 * are performed.
 * 
*/


/**
 * This method initializes a new Evolver object which facilitates the evolution of solutions to the problem.  First,
 * an initial population of representation type objects is generated at random.  Second, the fitness values of the
 * initial population are calculated and recorded.  Third, the population undergoes NUM_MATING_EVENTS mating events
 * iteratively improving the solutions towards the provided fitness function in run.cpp.  Each mating event consists of
 * size-TOURNAMENT_SIZE tournament selection, two-point crossover, and...(Kevin to finish).
 *
 * @tparam representation
 */
template<class representation>
Evolver<representation>::Evolver() {// This method is going to be similar to the main method (with the rest of the file being like main.h)

    for (int run = initRunNum; run < initRunNum + runs; run++) {// for each run
        //     sprintf(filename, "%srun%02d.dat", pathToOut, run);
        //     runStats.open(filename, ios::out);
        //     if (verbose) cmdLineRun(run, cout);
        initPop(); // Initialization the population
        for (int x = 0; x < SDAPop.size(); x++) {// check fitness of each SDA
            fit[x] = fitness(SDAPop[x]);// set SDA fitness in arrat at corresponding position
        }
        for (int x = 0;
             x < SDAPop.size(); x++) {//Generate the report ( can be combined with the fitness evaluation loop above ?)
            cout << "SDa" << x << endl;
            SDAPop[x].print(cout);
            cout << "Fitness of SDA: " << fit[x] << endl;
            //     report(runStats); // Initial Report
        }
        for (int x = 0; x < numGenerations; x++) {// perform the specefied number of mating events
            matingEvent();// perform the mating event with the current population
            //print fitness values
            for(int x = 0; x < SDAPop.size(); x++){//Generate the report ( can be combined with the fitness evaluation loop above ?)
            cout << "SDa" << x << endl;
            SDAPop[x].print(cout);
            cout << "Fitness of SDA: " << fit[x] << endl;
            //     report(runStats); // Initial Report
            }
        }
        //runStats.close();
        //reportBest(expStats);
        cout << "Done run " << run << ".  " << runs - (run - initRunNum + 1) << " more to go. " << endl;
    }
}//Evolver


// Initialize the population
// Calculate fitness of each member of the population
// Generate the first report
// Do the mating events
// Etc.

// AS A STARTING POINT
// This method will be similar to the main method in main.cpp
// The rest of this file will be like the methods in main.h

//}

/**
 * Kevin's first task:
 * 1. Write a dummy fitness method to be used on Graph's (i.e return the number of edges)
 * 
 * 2. Initialize a population of 10 representations (think of this as SDAs)
 * kev note - What is meant by representations (Where is the initial pop being initialized)
 * 
 * 3. Print the fitness values of the population to console (before evolution)
 * 4. Run 100 mating events
 * 5. Print the fitness values of the population to console (after evolution) (each mating event)
*/