Graph Evolution Tool
by Michael Dubé, James Sargent, Kevin Olenic, and Sheridan Houghten

This package is for evolving networks towards a user-defined goal.  To use the tool you can first download the code from Github.  The code files contained in the GET directory facilitates the evolution of the networks.  To use this tool you will be modifying the config.h and run.cpp files in this directory.  The config.h header file is used to set global variables dictating the settings to use when evolving the networks.  The run.cpp code file is used to provide a fitness function to be used by the evolver and run the evolution.  Both files are documented to guide you in setting up the system and running it successfully.

The contents of the GET directory and how the classes interact with one another are outlined below.
The Graph Class (Graph.h and Graph.cpp) is used to store the network.
The 