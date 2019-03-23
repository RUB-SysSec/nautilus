target="/home/prakti/forksrv/instrument/php/sapi/cli/php"
if [ -f "$target" ]
then
	mkdir -p old_vuln old_no_vuln
	echo "Using $target to triage!"
	for f in outputs/signaled/*; do
		if ASAN_OPTIONS=halt_on_error=false:allow_addr2line=true:allocator_may_return_null=1 timeout -s KILL 20 $target $f 2>&1 | grep '==ERROR: Ad'; then
			echo -e "\e[0;31m$f\e[0m"
			mv $f old_vuln/
		else 
			mv $f old_no_vuln/
		fi
	done
else
	echo "$target not found"
fi
