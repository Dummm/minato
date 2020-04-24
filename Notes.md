# minato - Container Management System

## Container runtime + other stuff

### Container manager

*Imaginile sunt dezarhivate in **.minato/images/{name}:{reference}***

Manager-ul de containere trebuie sa:

- [ ] Creeze container-ul in sine (folosind un id generat si o imagine, poate mai multe pe viitor):
  - [x] Creeze directorul pentru container
  - [ ] Genereze config.json pentru stocat configurarile (UPDATE: Posibil sa fie un fisier 'hardcodat'. Si apoi folosita la run)
- [ ] Ruleze container-ul
  - [x] Sa monteze cu overlay noul filesystem
  - [x] Sa faca root-ul filesistem-ului vechi privat
  - [x] Sa faca bind pe root-ul filesistem-ului nou
  - [x] clone, chdir, put_old
  - [x] Faca pivot_root peste noul filesystem (sau chroot)
  - [x] chdir pe /
  - [x] Sa faca clean-up la final

### Bugs

- [ ] Iesirea din sh (probabil ca nu mai are output)
