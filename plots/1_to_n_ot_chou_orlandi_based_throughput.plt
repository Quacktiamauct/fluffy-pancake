set title "1-to-n OT"
set key default
set xlabel "Number of Messages"
set logscale x 2
set ylabel "Throughput in Messages/s"
set logscale y 2
set term pict2e color linewidth 2
set output "out/1_to_n_ot_chou_orlandi_based_throughput.tex"

set xtics ("$2^{1}$" 2, "$2^{2}$" 4, "$2^{3}$" 8, "$2^{4}$" 16, "$2^{5}$" 32, "$2^{6}$" 64, "$2^{7}$" 128, "$2^{8}$" 256, "$2^{9}$" 512, "$2^{10}$" 1024, "$2^{11}$" 2048, "$2^{12}$" 4096, "$2^{13}$" 8192, "$2^{14}$" 16384, "$2^{15}$" 32768, "$2^{16}$" 65536, "$2^{17}$" 131072, "$2^{18}$" 262144, "$2^{19}$" 524288, "$2^{20}$" 1048576, "$2^{21}$" 2097152, "$2^{22}$" 4194304, "$2^{23}$" 8388608, "$2^{24}$" 16777216, "$2^{25}$" 33554432, "$2^{26}$" 67108864, "$2^{27}$" 134217728, "$2^{28}$" 268435456, "$2^{29}$" 536870912, "$2^{30}$" 1073741824)

plot "data/1_to_n_ot_chou_orlandi_based.dat" using 1:3 title "Chou-Orlandi Based" with lines
