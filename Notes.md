# minato - Container Management System
## Container runtime + other stuff

# Container manager
*Imaginile sunt dezarhivate in **.minato/images/{name}:{reference}***

Manager-ul de containere trebuie sa:
- [ ] Creeze container-ul in sine (folosind un id generat si o imagine, poate mai multe pe viitor):
    - [x] Creeze directorul pentru container
    - [ ] Genereze config.json pentru stocat configurarile
- [ ] Ruleze container-ul
    - [x] Sa monteze cu overlay noul filesystem
    - [ ] Faca pivot_root peste noul filesystem (sau chroot)
        - chdir pe /
    - [ ] Sa faca clean-up la final
