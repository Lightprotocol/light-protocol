from datetime import datetime
import os
import random
import sys
import shlex, subprocess

note_array = []
f = open("local_nullifier_test_notes_1.txt", "r")
now = datetime.now()

current_time = now.strftime("%H:%M:%S")

log_file_name = "nullifer_test_logs"+current_time+".txt"

log_file = open(log_file_name,'w')
#result_file.write("unotesse ark_ff::Fp384;\n")
#log_file.close()
found_note = False
counter = 0
for line in f:
    x = line.split()
    if len(x) > 3:

        if x[0] == 'PLEASE':
            #print(line)
            note_array.append(line.split(" ",5)[-1].split('\n')[0])

#print(note_array)

#random.shuffle(note_array)

for note in note_array[0:60]:
    log_file.write("\n\n\n")
    log_file.write("\nwithdrawing note ")
    log_file.write(note)
    log_file.write("\n")
    system_call = "npm run-script run withdraw " + note + " FR6FWUZNbS6mjMTb9WfTgeRHso5KsugHmHGTAh4zaQfb"# >>" + log_file_name + ";"
    system_call = shlex.split(system_call)
    #print(system_call)
    #os.system(system_call) # && cd ../test_scripts
    # out = subprocess.Popen(['wc', '-l', 'notes.txt'],
    #        stdout=subprocess.PIPE,
    #        stderr=subprocess.STDOUT)
    # stdout,stderr = out.communicate()
    # print(out.communicate())
    res = subprocess.run(system_call, stdout=subprocess.PIPE, cwd="../Client-Js")
    log_file.write("".join( chr(x) for x in bytearray(res.stdout) ))
    # log_file.write("\n")

# for note in note_array[0:2]:
#     log_file.write("\n\n\n")
#     log_file.write("\nwithdrawing note ")
#     log_file.write(note)
#     log_file.write("\n")
#     system_call = "npm run-script run withdraw " + note + " FR6FWUZNbS6mjMTb9WfTgeRHso5KsugHmHGTAh4zaQfb"# >>" + log_file_name + ";"
#
#     system_call = shlex.split(system_call)
#     #
#
#
#     #print(system_call)
#     #os.system(system_call) # && cd ../test_scripts
#     # out = subprocess.Popen(['wc', '-l', 'notes.txt'],
#     #        stdout=subprocess.PIPE,
#     #        stderr=subprocess.STDOUT)
#     # stdout,stderr = out.communicate()
#     # print(out.communicate())
#
#     try:
#         res = subprocess.run(system_call, stdout=subprocess.PIPE, cwd="../Client-Js")
#         log_file.write("".join( chr(x) for x in bytearray(res.stdout) ))
#     except:
#         print(res)
#     # log_file.write("\n")
