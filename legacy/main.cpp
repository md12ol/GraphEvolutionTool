#include <iostream>
#include <cstdlib>
#include <cstdio>
#include <cmath>
#include <iomanip>
#include <string>

using namespace std;

// This block of code includes the filesystem package (which depends on OS)
#if defined(__cplusplus) && __cplusplus >= 201703L && defined(__has_include)
#if __has_include(<filesystem>)
#define GHC_USE_STD_FS


#include <filesystem>
#include <thread>

namespace filesystem = std::filesystem;
#endif
#endif
#ifndef GHC_USE_STD_FS
#include "filesystem.hpp"
namespace filesystem = ghc::filesystem;
#endif

// Include the other files needed
#include "setu.h"
#include "stat.h"
#include "../BitSprayer/Bitsprayer.h"
#include "Graph.h"

/*************************algorithm controls******************************/
//#define PL 16
int PL = 16;
#define NSE 1
//int NSE = 5;
#define alpha 0.3
#define mepl 3              //  Minimum epidemic length
#define rse 5               //  Re-try short epidemics
#define FTL 50              //  Final test length
#define verbose true
#define runs 30
//#define RNS 9120783
int RNS;
#define tsize 5

int verts;
//int mevs;
//int popsize;
int states;
int MNM;
//#define verts 128
#define mevs 250000
#define popsize 250
//#define states 8
//#define MNM 7

#define RIs 100
#define RE ((long)mevs / RIs)
//int RE;
#define P0UPS 2
//int P0UPS;
#define P0INT ((long)mevs/P0UPS)
//int P0INT;
#define omega 0.5 // DiffChar
#define ent_thres 1.0 // Necrotic Filter
bool *dead;
double worst_fit;

static vector<double> edgBnds = {1, 7};
static vector<int> wghtBnds = {4, 16};
static double zeroProb = 0.90;
static bool diagFill = true;

/**************************Variable dictionary************************/
Bitsprayer *bPop;
double *fit; //  Fitness array
double *p0fit; //  Patient zero fitnesses
int *dx;   //  Sorting index
double *PD; //  Profile dictionary
int fitFun;          //  Profile?
int patient0; //  patient zero
Graph dublin_graph;

/****************************Procedures********************************/
void initalg(const char *pLoc); // initialize the algorithm
void initpop();                         // initialize population
void matingevent();                     // run a mating event
void report(ostream &aus);              // make a statistical report
void reportbest(ostream &aus, ostream &difc);
void createReadMe(ostream &aus);
void cmdLineIntro(ostream &aus);
void cmdLineRun(int run, ostream &aus);
double fitness(Bitsprayer &A, bool finalTest);
double fitnessBit(Graph &G, bool finalTest);
int patientZeroUpdate(int mev, ostream &patient0out, bool finalUpdate);

/****************************Main Routine*******************************/

int main(int argc, char *argv[]) {
    /**
     * Output Root, Mode, Verts, States, Muts, Profile Number, Run0, RNS
     */

    fstream best, dchar, stat, readme, patient0out; // statistics, best structures

    srand48(3);
    string str = "2 4 0 0 0 1 1 5 5 0 0 0 0 0 0 0 1 1 5 5 5 5 5 5 5 5 5 5 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 1 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 1 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 5 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 ";
    stringstream ss(str);
    string s;
    vector<int> weights;
    weights.reserve(1000000);
    while (ss >> s) {
        weights.push_back(stoi(s));
    }
    Graph G(128);
    G.fill(weights, true);
    G.print(cout);
    int cnt;
    vector<int> profile;
    vector<int> trials(NSE);
    double accu;
    for (int en = 0; en < NSE; en++) {
        cnt = 0;
        do {
            profile = G.SIR(alpha, 3);
//                for (int p : profile){
//                    cout<<p<<" ";
//                }
//                cout<<endl;
            cnt++;
        } while (profile.size() - 1 < mepl && cnt < rse);
        trials[en] = (double) profile.size() - 1;
    }
    for (double trial: trials) { // loop over trials
        accu += trial;
    }
    accu = accu / NSE;
    cout << accu << endl;


    int two = 2;
    two = 2 + 0;

    char fn[70];                                           // file name construction buffer
    char *outLoc = new char[60];
    char *pLoc = new char[50];
    char *outRoot = argv[1];
    fitFun = atoi(argv[2]); //  0 - Epidemic Length, 1 - Profile Matching
    verts = atoi(argv[3]);
    states = atoi(argv[4]);
    MNM = atoi(argv[5]);
    int run0 = atoi(argv[7]);
    RNS = atoi(argv[8]);

    bPop = new Bitsprayer[popsize];
    fit = new double[popsize];
    p0fit = new double[verts];
    dx = new int[popsize];
    dead = new bool[popsize];

    int pNum = -1;
    if (fitFun == 0) {
        sprintf(outLoc, "%sEDBit%d w %dS, %dM/", outRoot, verts, states, MNM);
    } else if (fitFun == 1) {
        pNum = atoi(argv[6]);
        if (pNum == 0) {
            PL = 30;
            verts = 200;
            sprintf(outLoc, "%sPMBit%s on P%s w %dS, %dM/", outRoot, "DUB", "D", states, MNM);
        } else {
            sprintf(outLoc, "%sPMBit%d on P%d w %dS, %dM/", outRoot, verts, pNum, states, MNM);
        }
        PD = new double[PL + 1];
    } else if (fitFun == 2) {
        verts = 200;
        sprintf(outLoc, "%sNMBit%s w %dS, %dM/", outRoot, "DUB", states, MNM);
        dublin_graph.fill("./dublin_graph.dat");
    } else {
        cout << "ERROR! No outLoc for this fitFun!" << endl;
    }

    filesystem::create_directory(outLoc);

    sprintf(pLoc, "./Profiles/Profile%d.dat", pNum);
    initalg(pLoc);
    sprintf(fn, "%sbest%02d.lint", outLoc, run0 + 1);
    best.open(fn, ios::out);
//    sprintf(fn, "%sdifc.dat", outLoc);
//    dchar.open(fn, ios::out);
    sprintf(fn, "%sreadme.dat", outLoc);
    readme.open(fn, ios::out);
    createReadMe(readme);
    readme.close();
    sprintf(fn, "%spatient0%02d.dat", outLoc, run0 + 1);
    patient0out.open(fn, ios::out);
    patient0out << "Patient Zero Update Info" << endl;

//    char* thing = new char[50];
//    sprintf(thing, "./jamesGraph_new.dat");
//    fstream inp;
//    inp.open(thing, ios::in);
//    int size = verts * (verts - 1) / 2;
//    vector<int> weights;
//    weights.reserve(size);
//    char buf[20];
//
//    for (int i = 0; i < size; i++) {
//        inp.getline(buf, 19);
//        weights.push_back(atoi(buf));
//    }
//
//    inp.close();
//    delete[] thing;
//
//    Graph G(verts);
//    G.fill(weights);
//    double sum = 0.0;
//    vector<int> prof;
//    double bb = 0.0;
//    for (int i = 0; i < 1000; ++i) {
//        prof = G.SIR(alpha,0);
//        sum += prof.size()-1;
//        if (prof.size()>bb){
//            bb = prof.size()-1;
//        }
//        for(int i: prof){
//            cout<<i<<" ";
//        }
//        cout<<endl;
//    }
//    cout<<"Average: " << (double)(sum/1000)<<endl;
//    cout<<"Best: " << (double)(bb)<<endl;

    if (verbose) {
        cmdLineIntro(cout);
    } else {
        cout << "Started" << endl;
    }

    for (int run = run0; run < run0 + 1; ++run) {
        if (fitFun != 2) {
            patient0 = (int) lrand48() % verts;
        }
        sprintf(fn, "%srun%02d.dat", outLoc, run + 1); // File name
        stat.open(fn, ios::out);
        initpop();
        if (verbose) cmdLineRun(run, cout);
        stat << left << setw(4) << 0;
        report(stat); // report the statistics of the initial population
        if (fitFun != 2) {
            patient0out << "Run # " << run + 1 << endl;
            patientZeroUpdate(0, patient0out, false);
        }
        for (int mev = 0; mev < mevs; mev++) { // do mating events
            matingevent(); // run a mating event
            if (fitFun != 2 && (mev + 1) < P0UPS * P0INT && (mev + 1) % P0INT == 0) {
                patientZeroUpdate(mev, patient0out, false);
            }
            if ((mev + 1) % RE == 0) { // Time for a report?
                if (verbose) {
                    cout << left << setw(5) << run + 1;
                    cout << left << setw(4) << (mev + 1) / RE;
                }
                stat << left << setw(4) << (mev + 1) / RE;
                report(stat); // report statistics
            }
        }
        patient0out << endl;
        stat.close();
//        reportbest(best, dchar);
        reportbest(best, cout);
        cout << "Done run " << run + 1 << " of " << runs << endl;
    }

    patient0out.close();
    best.close();
    dchar.close();
    delete[] outLoc;
    delete[] pLoc;
    delete[] bPop;
    delete[] fit;
    delete[] p0fit;
    delete[] dx;
    delete[] dead;
    delete[] PD;
    return (0); // keep the system happy
}

void createReadMe(ostream &aus) {
    aus << "This file contains the info about the files in this folder." << endl;
    aus << "Graph Evolution Tool." << endl;
    if (fitFun == 0) {
        aus << "Fitness Function: Epidemic Length" << endl;
    } else if (fitFun == 1) {
        aus << "Fitness Function: Profile Matching" << endl;
        aus << "Profile: ";
        for (int i = 0; i < PL + 1; i++) {
            aus << PD[i] << " ";
        }
        aus << endl;
    } else if (fitFun == 2) {
        aus << "Fitness Function: Network Matching" << endl;
        aus << "Network: Dublin Graph" << endl;
    } else {
        cout << "ERROR! No read me for this fitFun setting!" << endl;
    }
    aus << "Representation: Bitsprayer" << endl;
    aus << endl;
    aus << "The parameter settings are as follows: " << endl;
    aus << "Number of sample epidemics: " << NSE << endl;
    aus << "Alpha: " << alpha << endl;
    aus << "Minimum epidemic length: " << mepl << endl;
    aus << "Re-tries for short epidemics: " << rse << endl;
    aus << "Runs: " << runs << endl;
    aus << "Mating events: " << mevs << endl;
    aus << "Population size: " << popsize << endl;
    aus << "Number of vertices: " << verts << endl;
    aus << "Maximum number of mutations: " << MNM << endl;
    aus << "Tournament size: " << tsize << endl;
    aus << "Number of States: " << states << endl;
    aus << "Entropy Threshold for Necrotic Filter: " << ent_thres << endl;
    aus << "Decay strength for diffusion characters: " << omega << endl;
    aus << endl;
    aus << "The file descriptions are as follows: " << endl;
    aus << "best.lint -> the best fitness and it's associated data for each run";
    aus << endl;
    aus << "difc.dat -> the diffusion characters of the best graph for each run";
    aus << endl;
    aus << "run##.dat -> population statistics for each run" << endl;
    aus << "patient0.dat -> record of patient zeros" << endl;
}

void cmdLineIntro(ostream &aus) {
    aus << "Graph Evolution Tool." << endl;
    if (fitFun == 0) {
        aus << "Fitness Function: Epidemic Length" << endl;
    } else if (fitFun == 1) {
        aus << "Fitness Function: Profile Matching" << endl;
        aus << "Profile: ";
        for (int i = 0; i < PL + 1; i++) {
            aus << PD[i] << " ";
        }
        aus << endl;
    } else if (fitFun == 2) {
        aus << "Fitness Function: Network Matching" << endl;
        aus << "Network: Dublin Graph" << endl;
    } else {
        cout << "ERROR! No read me for this fitFun setting!" << endl;
    }
    aus << "Representation: Bitsprayer" << endl;
    aus << "Check readme.dat for more information about parameters/output.";
    aus << endl;
}

void cmdLineRun(int run, ostream &aus) {
    aus << endl << "Beginning Run " << run + 1 << " of " << runs << endl;
    aus << left << setw(5) << "Run";
    aus << left << setw(4) << "RI";
    aus << left << setw(10) << "Mean";
    aus << left << setw(12) << "95% CI";
    aus << left << setw(10) << "SD";
    aus << left << setw(8) << "Best";
    aus << endl;
    aus << left << setw(5) << run + 1;
    aus << left << setw(4) << "0";
}

void initalg(const char *pLoc) { // initialize the algorithm
    fstream inp;  // input file
    char buf[20]; // input buffer
    int val;

    srand48(RNS); // read the random number seed
    if (fitFun == 1) {
        inp.open(pLoc, ios::in); // open input file
        for (int i = 0; i < PL; i++) {
            PD[i] = 0; // pre-fill missing values
        }
        PD[0] = 1; // put in patient zero
        for (int i = 0; i < PL; i++) {                                     // loop over input values
            inp.getline(buf, 19);             // read in the number
            val = strtod(buf, nullptr);
            PD[i + 1] = val * ((double) verts / 128); // translate the number
        }
        inp.close();
    }
    for (int i = 0; i < popsize; ++i) {
        bPop[i] = *new Bitsprayer(states, zeroProb);
    }
}

int patientZeroUpdate(int mev, ostream &patient0out, bool finalUpdate) {
    if (finalUpdate) {
        double bestFit = (fitFun == 0 ? 0 : MAXFLOAT);
        int bestP0 = -1;
        int bestBit = -1;
        double oneFit;
        int origP0 = patient0;
        for (int i = 0; i < verts; i++) { // For every potential patient zero
            patient0 = i;
            for (int j = 0; j < popsize; j++) {
                if (patient0 == origP0) {
                    oneFit = fit[j];
                } else {
                    oneFit = fitness(bPop[j], false);
                }
                if (fitFun == 0 && oneFit > bestFit) {
                    bestP0 = i;
                    bestBit = j;
                    bestFit = oneFit;
                } else if ((fitFun == 1 || fitFun == 2) && oneFit < bestFit) {
                    bestP0 = i;
                    bestBit = j;
                    bestFit = oneFit;
                }
            }
        }
        patient0 = bestP0;
        fit[bestBit] = bestFit;
        for (int b = 0; b < popsize; ++b) {
            if (b != bestBit) {
                fit[b] = fitness(bPop[b], false);
            }
        }
    } else {
        for (int i = 0; i < verts; i++) { // For every potential patient zero
            patient0 = i;
            double sumTotal = 0.0;
            for (int j = 0; j < popsize; j++) {
                sumTotal += fitness(bPop[j], false);
            }
            p0fit[i] = sumTotal / popsize;
        }
        patient0 = 0;
        for (int k = 0; k < verts; k++) {
            if (fitFun == 0) {
                if (p0fit[k] > p0fit[patient0]) {
                    patient0 = k;
                }
            } else if (fitFun == 1 || fitFun == 2) {
                if (p0fit[k] < p0fit[patient0]) {
                    patient0 = k;
                }
            }
        }
        for (int b = 0; b < popsize; ++b) {
            fit[b] = fitness(bPop[b], false);
        }
    }

    patient0out << "The best p0 is " << patient0;
    patient0out << " at mating event " << mev + 1 << endl;
    return patient0;
}

bool necroticFilter(Bitsprayer &A) { // True means dead.
    int size = verts * (verts - 1) / 2;
    vector<int> vals;
    vals.reserve(size);

    vals = A.getBitsVec(size);

    int totWeight = 0;
    int totEdges = 0;
    for (int v: vals) {
        totWeight += v;
        if (v > 0) {
            totEdges++;
        }
    }
//    return false;
    if (totEdges < edgBnds[0] * verts || totEdges > edgBnds[1] * verts) {
        return true;
    }
    if (totWeight < wghtBnds[0] * verts || totWeight > wghtBnds[1] * verts) {
        return true;
    }
    return false;

//    double En = 0.0;
//    int binSize = 10;
//    int numBins = (int) pow(2, binSize); // TODO: Update
//    double counts[numBins];
//    int vals = Qz / binSize;
//    for (int i = 0; i < numBins; i++) {
//        counts[i] = 0.0;
//    }
//    for (int i = 0; i < Qz; i += binSize) {
//        counts[bitsToInt(Q + i, binSize)]++;
//    }
//    for (int i = 0; i < numBins; i++) {
//        counts[i] /= vals;
//    }
//    for (int i = 0; i < numBins; i++) {
//        if (counts[i] > 0.0) {
//            En += -counts[i] * log(counts[i]);
//        }
//    }
//    En /= log(2);
//    return En < ent_thres;
}


double fitnessBit(Graph &G, bool finalTest) { // compute the fitness
    int len;       // maximum, length, and total removed
    int cnt;                 // counter for tries
    vector<int> profile; // profile variable
    double trials[(finalTest ? FTL : NSE)];      // stores squared error for each trial
    int en;                  // epidemic number
    double delta;            // difference between profile and trial
    double accu = 0.0;       // accumulator

    if (fitFun == 0) {
        int tests = (finalTest ? FTL : NSE);
//        if (tests>NSE){
//            cout<<tests<<" Epidemics Performed"<<endl;
//        }
        for (en = 0; en < tests; en++) {
            cnt = 0;
            do {
                profile = G.SIR(alpha, patient0);
//                for (int p : profile){
//                    cout<<p<<" ";
//                }
//                cout<<endl;
                cnt++;
            } while (profile.size() - 1 < mepl && cnt < rse);
            trials[en] = (double) profile.size() - 1;
        }
        for (double trial: trials) { // loop over trials
            accu += trial;
        }
        accu = accu / tests;
    } else if (fitFun == 1) {
        for (en = 0; en < (finalTest ? FTL : NSE); en++) { // loop over epidemics
            cnt = 0;
            do {
                profile = G.SIR(alpha, patient0);
                cnt++;
            } while (profile.size() - 1 < mepl && cnt < rse);
            trials[en] = 0; // zero the current squared error
            len = (int) profile.size() - 1;
            for (int i = 0; i < PL + 1; i++) { // loop over time periods
                if (i < len) {
                    delta = profile[i] - PD[i];
                } else {
                    delta = PD[i];
                }
                trials[en] += delta * delta;
            }
            trials[en] = sqrt(trials[en] / (PL + 1)); // convert to RMS error
        }

        for (double trial: trials) {
            accu += trial;
        }
        accu = accu / NSE;
    } else if (fitFun == 2) {
        accu = G.hammy_distance(dublin_graph);
    }
    return accu; // return the fitness value
}

double fitness(Bitsprayer &A, bool finalTest) {
    int size = verts * (verts - 1) / 2;
    vector<int> vals;
    vals.reserve(size);

    vals = A.getBitsVec(size);
    Graph G(verts);
    G.fill(vals, diagFill);

    double fi = fitnessBit(G, finalTest);
    return fi;
}

void initpop() { // initialize population
    int count = 0;
    for (int i = 0; i < popsize; i++) {
        dead[i] = false;
        do {
            bPop[i].randomize();
            count++;
        } while (necroticFilter(bPop[i]));
        fit[i] = fitness(bPop[i], false);
        dx[i] = i;
//        cout<<"DONE 1"<<endl;
    }
    cout << "\n" << "Attempts: " << count << endl;
}

void matingevent() { // run a mating event
    int rp, sw;   // loop index, random position, swap variable
    int cp1, cp2; // crossover points
    Bitsprayer child1, child2;
    child1 = *new Bitsprayer(states, zeroProb);
    child2 = *new Bitsprayer(states, zeroProb);

    bool firstDead, secondDead;
    int deaths;
    do {
        // perform tournament selection
        if (fitFun == 0) {
            tselect(fit, dx, tsize, popsize); // lowest first
        } else if (fitFun == 1 || fitFun == 2) {
            Tselect(fit, dx, tsize, popsize); // highest first
        }

        child1.copy(bPop[dx[tsize - 2]]);
        child2.copy(bPop[dx[tsize - 1]]);
        child1.twoPtCrossover(child2);
        rp = (int) lrand48() % MNM + 1;
        child1.mutate(rp);
        rp = (int) lrand48() % MNM + 1;
        child2.mutate(rp);

        firstDead = necroticFilter(child1);
        secondDead = necroticFilter(child2);
        deaths = 0;
        if (firstDead || secondDead) {
            for (int i = 0; i < popsize; ++i) {
                if (dead[i]) {
                    deaths++;
                }
            }
        }
        if (firstDead) {
            deaths++;
        }
        if (secondDead) {
            deaths++;
        }
    } while (deaths > 0.8 * popsize);

    bPop[dx[0]].copy(child1);
    bPop[dx[1]].copy(child2);
    dead[dx[0]] = false;
    dead[dx[1]] = false;
    double fitVal = 0.0;
    if (firstDead) {
        dead[dx[0]] = true;
        fit[dx[0]] = worst_fit;
    } else {
        fitVal = fitness(bPop[dx[0]], false);
        if (fitFun == 2) {
            if (fitVal > 1.1 * worst_fit) {
                cout << fitVal << 1.1 * fitVal << endl;
                worst_fit = 1.1 * fitVal;
                for (int i = 0; i < popsize; i++) {
                    if (dead[i]) {
                        fit[i] = worst_fit;
                    }
                }
            } else {
                fit[dx[0]] = fitVal;
            }
        }
    }
    if (secondDead) {
        dead[dx[1]] = true;
        fit[dx[1]] = worst_fit;
    } else {
        fitVal = fitness(bPop[dx[1]], false);
        if (fitFun == 2) {
            if (fitVal > 1.1 * worst_fit) {
                cout << fitVal << 1.1 * fitVal << endl;
                worst_fit = 1.1 * fitVal;
                for (int i = 0; i < popsize; i++) {
                    if (dead[i]) {
                        fit[i] = worst_fit;
                    }
                }
            } else {
                fit[dx[1]] = fitVal;
            }
        }
    }
}

void report(ostream &aus) { // make a statistical report
    dset D;
    int deaths = 0;

    for (int i = 0; i < popsize; ++i) {
        if (dead[i]) {
            deaths++;
        }
    }
    double good_fit[popsize - deaths];
    for (double &i: good_fit) {
        i = 0.0;
    }
    int cnt = 0;
    for (int i = 0; i < popsize; i++) {
        if (!dead[i]) {
            good_fit[cnt++] = fit[i];
        }
    }

    int cntcnt = 0;
    for (int i = 0; i < popsize; ++i) {
        if (necroticFilter(bPop[i])) {
            cntcnt++;
        }
    }

    D.add(good_fit, popsize - deaths);
    if (fitFun == 0) {
        if (D.Rmin() != worst_fit) {
            worst_fit = D.Rmin();
            for (int i = 0; i < popsize; i++) {
                if (dead[i]) {
                    fit[i] = worst_fit;
                }
            }
        }
    } else if (fitFun == 1 || fitFun == 2) {
        if (D.Rmax() > worst_fit) {
            worst_fit = D.Rmax() * 1.1;
            for (int i = 0; i < popsize; i++) {
                if (dead[i]) { // update max
                    fit[i] = worst_fit;
                }
            }
        }
    }

    aus << left << setw(10) << D.Rmu();
    aus << left << setw(12) << D.RCI95();
    aus << left << setw(10) << D.Rsg();
    if (fitFun == 0) {
        aus << left << setw(8) << D.Rmax() << "\t";
    } else if (fitFun == 1 || fitFun == 2) {
        aus << left << setw(8) << D.Rmin() << "\t";
    }
    aus << "Dead: " << deaths << "\t" << worst_fit << endl;
    if (verbose) {
        cout << left << setw(10) << D.Rmu();
        cout << left << setw(12) << D.RCI95();
        cout << left << setw(10) << D.Rsg();
        if (fitFun == 0) {
            cout << left << setw(8) << D.Rmax() << "\t";
        } else if (fitFun == 1 || fitFun == 2) {
            cout << left << setw(8) << D.Rmin() << "\t";
        }
        cout << "Dead: " << deaths << "\t" << worst_fit << endl;
    }
    int b = -1;
    double best_fit = MAXFLOAT;
    for (int i = 0; i < popsize; i++) {
        if (fit[i] < best_fit && !dead[i]) {
            best_fit = fit[i];
            b = i;
        }
    }
    cout << "Best Saved: " << best_fit << "; Calculated: " << fitness(bPop[b], false);
}

void reportbest(ostream &aus, ostream &difc) { // report the best graph
    int b;          // loop indices and best pointer
    Graph G(verts);
    double En;
    double M[verts][verts];
    double Ent[verts];

//    for (int i = 0; i < popsize; ++i) {
//        if (!dead[i]){
//            fit[i] = fitness(*bPop[i], true);
//        }
//    }

    b = -1;
    double best_fit = MAXFLOAT;
    if (fitFun == 0) { // Epidemic Length
        for (int i = 1; i < popsize; i++) {
            if (fit[i] > fit[b] && !dead[i]) {
                b = i; // find best fitness
            }
        }
    } else if (fitFun == 1 || fitFun == 2) { // Profile Matching or Network Matching
        for (int i = 0; i < popsize; i++) {
            if (fit[i] < best_fit && !dead[i]) {
                b = i; // find best fitness
                best_fit = fit[i];
            }
        }
    }

    double fitCheck = fitness(bPop[b], false);
    if (fit[b] != fitCheck) {
        cout << "ERROR!!!  Fitness not what is expected!" << endl;
    }
    if (dead[b]) {
        cout << "ERROR!!!  Best is dead!" << endl;
    }
    if (necroticFilter(bPop[b])) {
        cout << "ERROR!!!  Best fails necrotic filter!!" << endl;
    }
    cout << "Stored Fitness: " << fit[b] << "; Calculated Fitness: " << fitCheck;
    aus << fit[b] << " -fitness" << endl;
    // Write the SDA
    aus << "Self-Driving Automata" << endl;
    bPop[b].print(aus);
    int size = verts * (verts - 1) / 2;
    bPop[b].printBitsVec(size, aus);
    vector<int> vals = bPop[b].getBitsVec(size);
    int vIdx = 0;
    G.fill(vals, diagFill);

    aus << "Graph" << endl;
    G.print(aus);
    aus << endl;
    //TODO: Later, fix.
//    for (int i = 0; i < G.size(); i++) {
//        G.DiffChar(i, omega, M[i]);
//    }
//
//    for (int i = 0; i < G.size(); i++) {             // loop over vertices
//        En = 0.0; // prepare En for normalizing
//        for (int j = 0; j < G.size(); j++) {
//            En += M[i][j]; // total the amount of gas at vertex i
//        }
//        for (int j = 0; j < G.size(); j++) {
//            M[i][j] /= En; // normalize so sum(gas)=1
//        }
//        En = 0.0; // zero the entropy accumulator
//        for (int j = 0; j < G.size(); j++) { // build up the individual entropy terms
//            if (M[i][j] > 0) {
//                En += -M[i][j] * log(M[i][j]); // this is entropy base E
//            }
//        }
//        Ent[i] = En / log(2); // convert entropy to Log base 2
//    }
//
//    // Now sort the entropy vector
//    bool more = false; // no swaps
//    do { // swap out-of-order entries
//        more = false;
//        for (int i = 0; i < G.size() - 1; i++) {
//            if (Ent[i] < Ent[i + 1]) {
//                En = Ent[i];
//                Ent[i] = Ent[i + 1];
//                Ent[i + 1] = En; // swap
//                more = true;     // set the flag that a swap happened
//            }
//        }
//    } while (more); // until in order
//
//    difc << Ent[0]; // output first entropy value
//    for (int i = 1; i < G.size(); i++) {
//        difc << " " << Ent[i]; // output remaining values
//    }
//    difc << endl; // end line
}
