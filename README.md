### PERFORMANCE
res: 1280x720

gpu: 1050ti

objects: 125 spheres in grid pattern

| depth      |  fps |
| ---------- | ---- |
| 0 (no bvh) |  11  |
| 1          |  40  |
| 5          |  30  |

the increase in fps when the bvh is at depth 1 is probably from the ray disregarding the entire scene when it does not intersect the root node
