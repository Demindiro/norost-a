========
Task I/O
========

 ::

      Client      |      Global kernel      |      Server

            +-----------+             +-----------+
            |           |   Request   |           |
            |           |---->->->----|           |
            |           |             |           |
            +-----------+             +-----------+
                                            |
                                            v
                                            |
            +-----------+             +-----------+
            |           |   Response  |           |
            |           |----<-<-<----|           |
            |           |             |           |
            +-----------+             +-----------+

