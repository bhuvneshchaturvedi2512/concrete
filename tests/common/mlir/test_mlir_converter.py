"""Test file for conversion to MLIR"""
# pylint: disable=no-name-in-module,no-member
import itertools

import pytest
from mlir.ir import IntegerType
from zamalang import compiler
from zamalang.dialects import hlfhe

from hdk.common.data_types.integers import Integer
from hdk.common.data_types.values import ClearValue, EncryptedValue
from hdk.common.mlir import V0_OPSET_CONVERSION_FUNCTIONS, MLIRConverter
from hdk.hnumpy.compile import compile_numpy_function


def add(x, y):
    """Test simple add"""
    return x + y


def sub(x, y):
    """Test simple sub"""
    return x - y


def mul(x, y):
    """Test simple mul"""
    return x * y


def sub_add_mul(x, y, z):
    """Test combination of ops"""
    return z - y + x * z


def ret_multiple(x, y, z):
    """Test return of multiple values"""
    return x, y, z


def ret_multiple_different_order(x, y, z):
    """Test return of multiple values in a different order from input"""
    return y, z, x


def datagen(*args):
    """Generate data from ranges"""
    for prod in itertools.product(*args):
        yield prod


@pytest.mark.parametrize(
    "func, args_dict, args_ranges",
    [
        (
            add,
            {
                "x": EncryptedValue(Integer(64, is_signed=False)),
                "y": ClearValue(Integer(32, is_signed=False)),
            },
            (range(0, 8), range(1, 4)),
        ),
        (
            add,
            {
                "x": ClearValue(Integer(32, is_signed=False)),
                "y": EncryptedValue(Integer(64, is_signed=False)),
            },
            (range(0, 8), range(1, 4)),
        ),
        (
            add,
            {
                "x": EncryptedValue(Integer(7, is_signed=False)),
                "y": EncryptedValue(Integer(7, is_signed=False)),
            },
            (range(7, 15), range(1, 5)),
        ),
        (
            sub,
            {
                "x": ClearValue(Integer(8, is_signed=False)),
                "y": EncryptedValue(Integer(7, is_signed=False)),
            },
            (range(5, 10), range(2, 6)),
        ),
        (
            mul,
            {
                "x": EncryptedValue(Integer(7, is_signed=False)),
                "y": ClearValue(Integer(8, is_signed=False)),
            },
            (range(1, 5), range(2, 8)),
        ),
        (
            mul,
            {
                "x": ClearValue(Integer(8, is_signed=False)),
                "y": EncryptedValue(Integer(7, is_signed=False)),
            },
            (range(1, 5), range(2, 8)),
        ),
        (
            sub_add_mul,
            {
                "x": EncryptedValue(Integer(7, is_signed=False)),
                "y": EncryptedValue(Integer(7, is_signed=False)),
                "z": ClearValue(Integer(7, is_signed=False)),
            },
            (range(0, 8), range(1, 5), range(5, 12)),
        ),
        (
            ret_multiple,
            {
                "x": EncryptedValue(Integer(7, is_signed=False)),
                "y": EncryptedValue(Integer(7, is_signed=False)),
                "z": ClearValue(Integer(7, is_signed=False)),
            },
            (range(1, 5), range(1, 5), range(1, 5)),
        ),
        (
            ret_multiple_different_order,
            {
                "x": EncryptedValue(Integer(7, is_signed=False)),
                "y": EncryptedValue(Integer(7, is_signed=False)),
                "z": ClearValue(Integer(7, is_signed=False)),
            },
            (range(1, 5), range(1, 5), range(1, 5)),
        ),
    ],
)
def test_mlir_converter(func, args_dict, args_ranges):
    """Test the conversion to MLIR by calling the parser from the compiler"""
    dataset = datagen(*args_ranges)
    result_graph = compile_numpy_function(func, args_dict, dataset)
    converter = MLIRConverter(V0_OPSET_CONVERSION_FUNCTIONS)
    mlir_result = converter.convert(result_graph)
    # testing that this doesn't raise an error
    compiler.round_trip(mlir_result)


def test_hdk_encrypted_integer_to_mlir_type():
    """Test conversion of EncryptedValue into MLIR"""
    value = EncryptedValue(Integer(7, is_signed=False))
    converter = MLIRConverter(V0_OPSET_CONVERSION_FUNCTIONS)
    eint = converter.hdk_value_to_mlir_type(value)
    assert eint == hlfhe.EncryptedIntegerType.get(converter.context, 7)


@pytest.mark.parametrize("is_signed", [True, False])
def test_hdk_clear_integer_to_mlir_type(is_signed):
    """Test conversion of ClearValue into MLIR"""
    value = ClearValue(Integer(5, is_signed=is_signed))
    converter = MLIRConverter(V0_OPSET_CONVERSION_FUNCTIONS)
    int_mlir = converter.hdk_value_to_mlir_type(value)
    with converter.context:
        if is_signed:
            assert int_mlir == IntegerType.get_signed(5)
        else:
            assert int_mlir == IntegerType.get_unsigned(5)


def test_failing_hdk_to_mlir_type():
    """Test failing conversion of an unsupported type into MLIR"""
    value = "random"
    converter = MLIRConverter(V0_OPSET_CONVERSION_FUNCTIONS)
    with pytest.raises(TypeError, match=r"can't convert value of type .* to MLIR type"):
        converter.hdk_value_to_mlir_type(value)


# pylint: enable=no-name-in-module,no-member
