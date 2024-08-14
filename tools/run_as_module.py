#!/usr/bin/env python3

import argparse
import sys
import os.path

parser = argparse.ArgumentParser(
    prog='run_as_module',
    description='Run Python file as if importing from my.module',
)
parser.add_argument('module', metavar='MODULE', type=str)
parser.add_argument('file', metavar='FILE', type=str)

args = parser.parse_args()

module_name = args.module
file_name = args.file

if not os.path.exists(file_name) or not os.path.isfile(file_name):
    print(f"Cannot open {file_name}")
    sys.exit(1)


from importlib.abc import InspectLoader, Loader, MetaPathFinder
from importlib.machinery import ModuleSpec
import importlib.util
import importlib
import sys
import os



class FindTargetModule(MetaPathFinder):
    """Resolve a given `module_name` to a given `file_name`"""

    def __init__(self, module_name, file_name):
        self.file_name = os.path.abspath(file_name)
        self.file_dir = os.path.dirname(file_name)

        self.module_name = module_name
        self.module_prefixes = set()

        module_name_arr = module_name.split('.')
        for i in range(len(module_name_arr)):
            self.module_prefixes.add(".".join(module_name_arr[:i+1]))

    def find_spec(self, fullname, path, target=None):
        if fullname == self.module_name:
            return importlib.util.spec_from_file_location(
                self.module_name, self.file_name
            )

        # For names that match a prefix of the module, load
        # a namespace-ish module
        if fullname in self.module_prefixes:
            spec = ModuleSpec(
                name = fullname, loader = LoadEmptyModule(),
            )
            spec.submodule_search_locations = [self.file_dir]
            return spec

        return None


class LoadEmptyModule(InspectLoader):
    """Load "empty" modules, like the Namespace loader"""

    def get_code(self, fullname):
        return compile('', '<string>', 'exec', dont_inherit=True)

    def get_source(self, fullname):
        return ''

    def is_package(self, fullname):
        return True
    

find_target_module = FindTargetModule(
    module_name=module_name,
    file_name=file_name
)
sys.meta_path.insert(0, find_target_module)


m = __import__(module_name, fromlist=[None])
