#!/bin/bash
spin[0]="-"
spin[1]="\\"
spin[2]="|"
spin[3]="/"

j=0
while [ $j -lt 3 ]; do
  for i in "${spin[@]}"
  do
        echo -ne "\b$i"
        sleep 0.1
  done
  j=$((j+1))
done