"""Test file for compilation configuration"""

from inspect import signature

import numpy
import pytest

from hdk.common.compilation import CompilationConfiguration
from hdk.common.data_types.integers import Integer
from hdk.common.values import EncryptedValue
from hdk.hnumpy.compile import compile_numpy_function_into_op_graph


def no_fuse(x):
    """No fuse"""
    return x + 2


def simple_fuse_not_output(x):
    """Simple fuse not output"""
    intermediate = x.astype(numpy.float64)
    intermediate = intermediate.astype(numpy.uint32)
    return intermediate + 2


@pytest.mark.parametrize(
    "function_to_trace,fused",
    [
        pytest.param(
            no_fuse,
            False,
            id="no_fuse",
        ),
        pytest.param(
            simple_fuse_not_output,
            True,
            id="simple_fuse_not_output",
            marks=pytest.mark.xfail(strict=True),
            # fails because it connot be compiled without topological optimizations
        ),
    ],
)
def test_enable_topological_optimizations(test_helpers, function_to_trace, fused):
    """Test function for enable_topological_optimizations flag of compilation configuration"""

    op_graph = compile_numpy_function_into_op_graph(
        function_to_trace,
        {
            param: EncryptedValue(Integer(32, is_signed=False))
            for param in signature(function_to_trace).parameters.keys()
        },
        iter([(1,), (2,), (3,)]),
        CompilationConfiguration(dump_artifacts_on_unexpected_failures=False),
    )
    op_graph_not_optimized = compile_numpy_function_into_op_graph(
        function_to_trace,
        {
            param: EncryptedValue(Integer(32, is_signed=False))
            for param in signature(function_to_trace).parameters.keys()
        },
        iter([(1,), (2,), (3,)]),
        CompilationConfiguration(
            dump_artifacts_on_unexpected_failures=False,
            enable_topological_optimizations=False,
        ),
    )

    graph = op_graph.graph
    not_optimized_graph = op_graph_not_optimized.graph

    if fused:
        assert not test_helpers.digraphs_are_equivalent(graph, not_optimized_graph)
        assert len(graph) < len(not_optimized_graph)
    else:
        assert test_helpers.digraphs_are_equivalent(graph, not_optimized_graph)
        assert len(graph) == len(not_optimized_graph)
