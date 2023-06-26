#include "Evolver.h"
#include "config.h"

double fitness(){
    return 10;
}

template<class representation>
Evolver<representation>::Evolver() {

    // Initialize the population
    // Calculate fitness of each member of the population
    // Generate the first report
    // Do the mating events
    // Etc.

    // AS A STARTING POINT
    // This method will be similar to the main method in main.cpp
    // The rest of this file will be like the methods in main.h

    if (BIGGER_BETTER){
        cout<<"WOO!"<<endl;
    }

}

/**
 * Kevin's first task:
 * 1. Write a dummy fitness method to be used on Graph's (i.e return the number of edges)
 * 2. Initialize a population of 10 representations (think of this as SDAs)
 * 3. Print the fitness values of the population to console (before evolution)
 * 4. Run 100 mating events
 * 5. Print the fitness values of the population to console (after evolution)
*/