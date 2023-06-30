/**
 * Users will be provided with defaults for various defined values and information on what options they have.
 * Can provide an initial network OR select from some subset.
 *
 *
 *
 *
 *
 *

 *
 *
 *
 * Number of Nodes =
 * Directed Graph =
 * Weighted Graph =
 * Adj Matrix Rep =
 * Number of Generations =
 * Population Size =
 *
 *
 *
 *
 *
*/

// If you want to maximize fitness set this to true.  Otherwise set it to false.
#define BIGGER_BETTER true

#define TRUE (int)1
#define FALSE (int)0
#define NUM_NODES (int)100 // SET BY USER
#define SDA_REP TRUE // SET BY USER
#define DIRECTED_GRAPH FALSE // SET BY USER
#define WEIGHTED_GRAPH FALSE // SET BY USER

#define ADJ_LIST_REP FALSE // SET BY USER

#if SDA_REP == FALSE
#define CHROMOSOME_LEN 256 // SET BY USER
#define EEOPS_REP TRUE
#else
#define EEOPS_REP FALSE
#endif

#if WEIGHTED_GRAPH == TRUE
#define MAX_WEIGHT (int) 10 // SET BY USER
#define MULTI_GRAPH FALSE // SET BY USER
#else
#define MAX_WEIGHT (int) 1
#define MULTI_GRAPH FALSE
#endif




/**
 * The following definitions are for setting the parameters of the evolutionary algorithm.
 * All of which can be modified by the user.
 */
#define POPULATION_SIZE (int)10         // SET BY USER
#define NUM_MATING_EVENTS (int)10000    // SET BY USER
#define TOURNAMENT_SIZE (int)7          // SET BY USER
#define CROSSOVER_RATE (float)0.5       // SET BY USER
#define DYNAMIC_MUTATION_RATE TRUE      // SET BY USER // Kevin, implement this in GETBase with Sigmoid function :)
#define MUTATION_RATE (float)0.05       // SET BY USER