set title "Garbled Circuits"
set key default
set xlabel "Number of Password bits"
set logscale x 2
set ylabel "Runtime"
set logscale y 2
set term pict2e color linewidth 2
set output "out/garbled_circuits_evaluate.tex"

set xtics ("$2^{1}$" 2, "$2^{2}$" 4, "$2^{3}$" 8, "$2^{4}$" 16, "$2^{5}$" 32, "$2^{6}$" 64, "$2^{7}$" 128, "$2^{8}$" 256, "$2^{9}$" 512, "$2^{10}$" 1024, "$2^{11}$" 2048, "$2^{12}$" 4096, "$2^{13}$" 8192, "$2^{14}$" 16384, "$2^{15}$" 32768, "$2^{16}$" 65536, "$2^{17}$" 131072, "$2^{18}$" 262144, "$2^{19}$" 524288, "$2^{20}$" 1048576, "$2^{21}$" 2097152, "$2^{22}$" 4194304, "$2^{23}$" 8388608, "$2^{24}$" 16777216, "$2^{25}$" 33554432, "$2^{26}$" 67108864, "$2^{27}$" 134217728, "$2^{28}$" 268435456, "$2^{29}$" 536870912, "$2^{30}$" 1073741824)

set ytics ("$4\\mu s$" 0.0039, "$8\\mu s$" 0.0078, "$16\\mu s$" 0.0156, "$31\\mu s$" 0.0312, "$62\\mu s$" 0.0625, "$125\\mu s$" 0.1250, "$250\\mu s$" 0.2500, "$500\\mu s$" 0.5000, "1 ms" 1, "2 ms" 2, "4 ms" 4, "8 ms" 8, "16 ms" 16, "32 ms" 32, "64 ms" 64, "128 ms" 128, "256 ms" 256, "512 ms" 512, "1.02 s" 1024, "2.05 s" 2048, "4.10 s" 4096, "8.19 s" 8192, "16.38 s" 16384, "32.77 s" 32768, "65.54 s" 65536, "131.07 s" 131072, "262.14 s" 262144, "524.29 s" 524288, "1,048.58 s" 1048576, "2,097.15 s" 2097152)

plot "data/garbled_circuits_evaluate.dat" using 1:2 title "Evaluate" with lines
