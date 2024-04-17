from typing import List, Optional


class UserProvidedConfig:
    """
    UserProvidedConfig is a class that represents the service configuration

    :param port: the port on which the service should running
    :param replicas: the number of replicas of the service
    :param cloud: the cloud on which the service should running
    :param workdir: the working directory of the service
    :param disk_size: the disk size of the service
    :param cpu: the CPU upper bound of the service
    :param memory: the memory upper bound of the service
    :param setup: the setup command of the service
    :param run: the run command of the service
    """

    def __init__(self, port: Optional[int] = None,
                 replicas: Optional[int] = None,
                 cloud: Optional[str] = None,
                 workdir: Optional[str] = None,
                 disk_size: Optional[int] = None,
                 cpu: Optional[str] = None,
                 memory: Optional[str] = None,
                 setup: Optional[str] = None,
                 run: Optional[str] = None) -> None: ...


class Dispatcher:
    """
    Dispatcher is a class that represents the service dispatcher, which is
    responsible for housing all the Servicing functionality
    """

    def __init__(self) -> None: ...

    def add_service(self, name: str,
                    config: Optional[UserProvidedConfig] = None) -> None:
        """
        Add a new service to the dispatcher

        :param name: the name of the service
        :param config: the configuration of the service
        """

    def remove_service(self, name: str) -> None:
        """
        Remove a service from the dispatcher

        :param name: the name of the service
        """

    def up(self, name: str) -> None:
        """
        Start a service

        :param name: the name of the service to start
        """

    def down(self, name: str) -> None:
        """
        Stop a service

        :param name: the name of the service to stop
        """

    def status(self, name: str, pretty: Optional[bool] = None) -> str:
        """
        Get the status of a service

        :param name: the name of the service
        :param pretty: whether to return the status in a pretty format
        :return: the status of the service in string format
        """

    def save(self, location: Optional[str] = None) -> None:
        """
        Save the dispatcher's cache

        :param location: the location of the cache, defaults to home directory
        """

    def save_as_b64(self) -> str:
        """
        Save the dispatcher's cache as a base64 string

        :return: the base64 string of the cache
        """

    def load(self, location: Optional[str] = None) -> None:
        """
        Load the dispatcher's cache

        :param location: the location of the cache, defaults to home directory
        """

    def load_as_b64(self, b64: str) -> None:
        """
        Load the dispatcher's cache from a base64 string

        :param b64: the base64 string of the cache
        """

    def list(self) -> List[str]:
        """
        List all the services

        :return: a list of all the services
        """
