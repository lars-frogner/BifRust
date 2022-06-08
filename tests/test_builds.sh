#!/bin/sh
set -e

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]:-$0}"; )" &> /dev/null && pwd 2> /dev/null; )"
PROJECT_DIR="$(dirname $SCRIPT_DIR)"

FEATURE_SETS=(
    'NONE'
    'corks'
    'tracing'
    'ebeam'
    'synthesis'
    'comparison'
    'cli'
    'cli,command-graph'
    'cli,statistics'
    'cli,json'
    'cli,pickle'
    'cli,netcdf'
    'cli,corks'
    'cli,corks,json'
    'cli,corks,pickle'
    'cli,corks,json,pickle'
    'cli,tracing'
    'cli,ebeam'
    'cli,ebeam,json'
    'cli,ebeam,pickle'
    'cli,ebeam,hdf5'
    'cli,ebeam,json,pickle,hdf5'
    'cli,synthesis'
    'cli,statistics,synthesis'
    'cli,netcdf,synthesis'
    'cli,statistics,json,pickle,hdf5,netcdf'
    'cli,statistics,json,pickle,hdf5,netcdf,comparison'
    'cli,statistics,json,pickle,hdf5,netcdf,tracing'
    'cli,statistics,json,pickle,hdf5,netcdf,tracing,comparison'
    'cli,statistics,json,pickle,hdf5,netcdf,corks,tracing'
    'cli,statistics,json,pickle,hdf5,netcdf,corks,tracing,comparison'
    'cli,statistics,json,pickle,hdf5,netcdf,corks,tracing,ebeam'
    'cli,statistics,json,pickle,hdf5,netcdf,corks,tracing,ebeam,comparison'
    'cli,statistics,json,pickle,hdf5,netcdf,corks,tracing,ebeam,synthesis'
    'cli,statistics,json,pickle,hdf5,netcdf,corks,tracing,ebeam,synthesis,comparison'
    'ALL'
)

for FEATURE_SET in "${FEATURE_SETS[@]}"; do
    if [[ "$FEATURE_SET" = "NONE" ]]; then
        FEATURE_MSG='no features'
        FEATURE_ARGS='--no-default-features'
    elif [[ "$FEATURE_SET" = "ALL" ]]; then
        FEATURE_MSG='all features'
        FEATURE_ARGS='--all-features'
    else
        FEATURE_MSG="features: $FEATURE_SET"
        FEATURE_ARGS="--no-default-features --features=$FEATURE_SET"
    fi
    echo -n "Building with $FEATURE_MSG... "
    OUTPUT="$(bash $PROJECT_DIR/setup_backstaff --no-to-all check $FEATURE_ARGS 2>&1)"

    if [[ ! "$?" = 0 ]]; then
        echo Failure >&2
        echo $OUTPUT >&2
        exit 1
    fi

    echo Success
done