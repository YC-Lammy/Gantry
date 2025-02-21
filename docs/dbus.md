Gantry API over DBus
====================
Gantry natively expose its API over DBus. It is registered under the well-known name *org.gantry.ThreeD*. The server service is served at **/org/gantry/server** with interface **org.gantry.server**.

Each instance of printer cretaed will be allocated its own path **/org/gantry/instance{index}** with interface **org.gantry.printer**.

# org.gantry.server
The server interface handles system operations such as creating instances and query system info.

|method|description|
|-----|-----|

# org.gantry.printer
The printer interface handles operations on individual printers.

|method|description|
|-----|-----|