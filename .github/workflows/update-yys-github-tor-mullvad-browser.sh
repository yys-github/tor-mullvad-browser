_r()
{
	echo "[$(date +"%Y-%m-%d %H:%M:%S")] Running [$@]..."
	if ! "$@" ; then
		echo "[$(date +"%Y-%m-%d %H:%M:%S")] ERROR: [$@] returns $?, please check!"
		exit 1
	fi
	echo "[$(date +"%Y-%m-%d %H:%M:%S")] Done [$@]."
}

cd "$GITHUB_WORKSPACE"
for remote in tor mullvad ; do
	echo Processing [$remote]...
	_r git remote add $remote https://gitlab.torproject.org/tpo/applications/$remote-browser.git
	_r git fetch --verbose --tags $remote
	# push one branch one time to avoid push >2gb limit
	for ref in $(git for-each-ref --format='%(refname)' refs/remotes/$remote/) ; do
		if [ "$ref" != "refs/remotes/$remote/HEAD" ] ; then
			echo Pushing [$remote]/[$branch]...
			branch=${ref##*/}
			# assert branch format correct
			if [ "refs/remotes/$remote/$branch" != "$ref" ] ; then
				exit 1
			fi
			_r git push --verbose origin "$ref:refs/heads/$branch"
			echo Done [$remote]/[$branch].
		fi
	done
	# do a final total push, include tags, exclude HEAD (may unintentionally change default branch of upstream)
	# developer notes: "^refs/remotes/$remote/HEAD" or "^refs/remotes/*/HEAD" won't exclude correctly, but "^refs/*/HEAD" will. maybe a bug.
	_r git push --verbose --tags origin "refs/remotes/$remote/*:refs/heads/*" "^refs/*/HEAD"
	echo Done [$remote].
done
