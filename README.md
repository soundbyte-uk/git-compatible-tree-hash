Git-compatible Tree Hashes
==========================

When working on a monorepo containing multiple different components, it can be helpful to figure out what's changed between different versions, in order to optimize out unnecessary builds and deployments that don't effect any change.

This involves identifying different subtrees of the repository which affect the different components.

In order to then detect changes, it's useful to be able to detect whether the selected subtree is identical between two versions or not.

The most efficient way to do this is to persist a hash which uniquely summarizes the contents of the whole tree that led to a particular deployment.

Git internally does this for the file tree belonging to each commit object, but:

* In this case, we don't want to hash the whole repository, only a selected subset of the trees and files.

* Often the final hash to describe a deployment must include generated files (for example, environmental configuration) as well as committed ones.

Nonetheless, Git's internal format is a convenient testable spec to design to, so this project attempts to produce an identical hash of a given directory tree to that which Git would produce if the same directory was in its index (`git write-tree`).


Prototype Progress
------------------

This prototype currently just hashes an entire directory, with no specific awareness of ignored files.

To be useful in real life, it will need to become part of a design which more clearly identifies which files to include or uninclude in a specific situation.
