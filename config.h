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

#define TRUE 1
#define FALSE 0
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